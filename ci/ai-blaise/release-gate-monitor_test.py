#!/usr/bin/env python3
"""Regression tests for the fail-closed release-gate monitor boundary."""

from __future__ import annotations

import os
from pathlib import Path
import stat
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
MONITOR = ROOT / "ci/ai-blaise/release-gate-monitor.sh"


class ReleaseGateMonitorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory(
            prefix="ai-blaise-release-gate-monitor-"
        )
        self.bin_directory = Path(self.temporary_directory.name) / "bin"
        self.bin_directory.mkdir()
        self.cargo_log = Path(self.temporary_directory.name) / "cargo.log"
        self.gh_log = Path(self.temporary_directory.name) / "gh.log"
        self.write_executable(
            "cargo",
            r"""
            #!/usr/bin/env bash
            set -euo pipefail
            printf '%s\n' "$*" >>"${FAKE_CARGO_LOG}"
            command_name="${!#}"
            maturity="${FAKE_MATURITY:-partial}"
            claimed_status="${FAKE_CLAIMED_STATUS:-alpha}"
            case "${command_name}" in
              check)
                printf 'feature_register_check\tpassed\nrows\t2\ncanonical_edges\t1\nsource_paths\t1\ncheck_paths\t1\nevidence_paths\t0\n'
                ;;
              summary)
                printf 'feature_register_summary\tvalid\nrows\t2\nmaturity\t%s\t2\nclaimed_status\t%s\t2\n' "${maturity}" "${claimed_status}"
                ;;
              release-gaps)
                printf 'feature_release_gaps\tblocked\n'
                printf 'release_evidence_verifier\tunimplemented\n'
                printf 'feature_release_gap_columns\tid\ttitle\tkind\tdisposition\tcanonical_ids\tmaturity\thistorical_claimed_status\tscope\tblockers\n'
                printf 'feature_release_gap\tA\tActive A\tproduct\tactive\t-\t%s\t%s\tBounded A\tTrusted release receipt absent\n' "${maturity}" "${claimed_status}"
                printf 'feature_release_gap\tB\tAlias B\tcapability\talias\tA\t%s\t%s\tHistorical alias\tCanonical A owns behavior\n' "${maturity}" "${claimed_status}"
                if [[ -n "${FAKE_EXTRA_RELEASE_RECORD:-}" ]]; then
                  printf '%s\n' "${FAKE_EXTRA_RELEASE_RECORD}"
                fi
                exit "${FAKE_RELEASE_EXIT:-1}"
                ;;
              *)
                printf 'unexpected cargo invocation: %s\n' "$*" >&2
                exit 97
                ;;
            esac
            """,
        )
        self.write_executable(
            "gh",
            r"""
            #!/usr/bin/env bash
            set -euo pipefail
            printf '%s\n' "$*" >>"${FAKE_GH_LOG}"
            expected_pr="${FAKE_EXPECTED_PR:-123}"
            if [[ "$*" != "pr checks --json name,state,bucket,workflow,link -- ${expected_pr}" ]]; then
              printf 'unit pass 1s https://example.invalid/table\n'
              exit 0
            fi
            case "${FAKE_GH_RESULT:-pass}" in
              pass)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"pass","workflow":"ci","link":"https://example.invalid/pass"}]'
                ;;
              pending)
                printf '%s\n' '[{"name":"unit","state":"IN_PROGRESS","bucket":"pending","workflow":"ci","link":"https://example.invalid/pending"}]'
                exit 8
                ;;
              fail)
                printf '%s\n' '[{"name":"unit","state":"FAILURE","bucket":"fail","workflow":"ci","link":"https://example.invalid/fail"}]'
                exit 1
                ;;
              skipping)
                printf '%s\n' '[{"name":"docs","state":"SKIPPED","bucket":"skipping","workflow":"ci","link":"https://example.invalid/skipped"}]'
                ;;
              cancelled)
                printf '%s\n' '[{"name":"unit","state":"CANCELLED","bucket":"cancel","workflow":"ci","link":"https://example.invalid/cancelled"}]'
                ;;
              unknown_bucket)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"unknown","workflow":"ci","link":"https://example.invalid/unknown"}]'
                ;;
              unknown_state)
                printf '%s\n' '[{"name":"unit","state":"MYSTERY","bucket":"pending","workflow":"ci","link":"https://example.invalid/unknown"}]'
                ;;
              empty)
                printf '%s\n' '[]'
                exit 1
                ;;
              no_output)
                exit 1
                ;;
              malformed)
                printf '%s\n' '{not-json'
                ;;
              invalid_schema)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"pass","workflow":"ci","link":"https://example.invalid/pass","conclusion":"SUCCESS"}]'
                ;;
              auth_failure)
                printf 'authentication failed\n' >&2
                exit 1
                ;;
              partial_auth_failure)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"pass","workflow":"ci","link":"https://example.invalid/pass"}]'
                exit 1
                ;;
              inconsistent_pending_exit)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"pass","workflow":"ci","link":"https://example.invalid/pass"}]'
                exit 8
                ;;
              unsupported_exit)
                printf '%s\n' '[{"name":"unit","state":"SUCCESS","bucket":"pass","workflow":"ci","link":"https://example.invalid/pass"}]'
                exit 2
                ;;
              *)
                exit 98
                ;;
            esac
            """,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write_executable(self, name: str, contents: str) -> None:
        path = self.bin_directory / name
        path.write_text(textwrap.dedent(contents).lstrip(), encoding="utf-8")
        path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def run_monitor(
        self, *arguments: str, **environment: str
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.update(
            {
                "PATH": f"{self.bin_directory}{os.pathsep}{env['PATH']}",
                "RELEASE_GATE_MONITOR_STATIC": "1",
                "FAKE_CARGO_LOG": str(self.cargo_log),
                "FAKE_GH_LOG": str(self.gh_log),
            }
        )
        env.update(environment)
        return subprocess.run(
            ["bash", str(MONITOR), *arguments],
            cwd=ROOT,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_historical_claims_cannot_enable_release_qualification(self) -> None:
        result = self.run_monitor(
            "--local-only", FAKE_CLAIMED_STATUS="production-ready"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("release_qualification=blocked", result.stdout)
        self.assertIn("release_evidence_verifier=unimplemented", result.stdout)
        self.assertNotIn("release_qualification=passed", result.stdout)

    def test_implemented_maturity_cannot_enable_release_qualification(self) -> None:
        result = self.run_monitor("--local-only", FAKE_MATURITY="implemented")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("release_qualification=blocked", result.stdout)
        self.assertNotIn("release_qualification=passed", result.stdout)
        self.assertEqual(
            self.cargo_log.read_text(encoding="utf-8").splitlines(),
            [
                "run --locked --quiet -p ai_blaise_feature_register -- check",
                "run --locked --quiet -p ai_blaise_feature_register -- summary",
                "run --locked --quiet -p ai_blaise_feature_register -- release-gaps",
            ],
        )
        self.assertFalse(self.gh_log.exists(), "--local-only must not query GitHub")

    def test_a_positive_release_gap_exit_is_rejected(self) -> None:
        result = self.run_monitor("--local-only", FAKE_RELEASE_EXIT="0")
        self.assertEqual(result.returncode, 1)
        self.assertIn(
            "release-gaps must return unqualified exit 1, got 0", result.stderr
        )

    def test_an_unexpected_release_record_is_rejected(self) -> None:
        result = self.run_monitor(
            "--local-only",
            FAKE_EXTRA_RELEASE_RECORD="release_qualification\tpassed",
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn(
            "release-gaps must preserve every validated feature as one ten-field blocked record",
            result.stderr,
        )

    def test_pr_check_supported_json_and_exit_statuses_are_preserved(self) -> None:
        cases = (
            (
                "pass",
                0,
                "pass=1\tpending=0\tskipping=0\tfail=0\tcancel=0",
            ),
            (
                "pending",
                0,
                "pass=0\tpending=1\tskipping=0\tfail=0\tcancel=0",
            ),
            (
                "fail",
                1,
                "pass=0\tpending=0\tskipping=0\tfail=1\tcancel=0",
            ),
        )
        for mode, expected_code, expected_summary in cases:
            with self.subTest(mode=mode):
                result = self.run_monitor("--pr", "123", FAKE_GH_RESULT=mode)
                self.assertEqual(result.returncode, expected_code, result.stderr)
                self.assertIn(expected_summary, result.stdout)
                self.assertEqual(
                    self.gh_log.read_text(encoding="utf-8").splitlines()[-1],
                    "pr checks --json name,state,bucket,workflow,link -- 123",
                )

    def test_pr_skipping_and_cancelled_checks_are_explicit(self) -> None:
        skipped = self.run_monitor("--pr", "123", FAKE_GH_RESULT="skipping")
        self.assertEqual(skipped.returncode, 0, skipped.stderr)
        self.assertIn("pass=0\tpending=0\tskipping=1\tfail=0\tcancel=0", skipped.stdout)
        self.assertIn(
            "SKIPPING\tSKIPPED\tci/docs\thttps://example.invalid/skipped",
            skipped.stdout,
        )

        cancelled = self.run_monitor("--pr", "123", FAKE_GH_RESULT="cancelled")
        self.assertEqual(cancelled.returncode, 1, cancelled.stderr)
        self.assertIn(
            "pass=0\tpending=0\tskipping=0\tfail=0\tcancel=1",
            cancelled.stdout,
        )
        self.assertIn(
            "CANCEL\tCANCELLED\tci/unit\thttps://example.invalid/cancelled",
            cancelled.stdout,
        )

    def test_pr_unknown_state_bucket_or_schema_is_rejected(self) -> None:
        for mode in ("unknown_bucket", "unknown_state", "invalid_schema"):
            with self.subTest(mode=mode):
                result = self.run_monitor("--pr", "123", FAKE_GH_RESULT=mode)
                self.assertEqual(result.returncode, 4)
                self.assertIn("invalid PR checks response", result.stderr)
                self.assertNotIn("release_gate_monitor_pr_checks", result.stdout)

    def test_pr_empty_or_malformed_responses_cannot_certify(self) -> None:
        for mode in ("empty", "no_output", "malformed"):
            with self.subTest(mode=mode):
                result = self.run_monitor("--pr", "123", FAKE_GH_RESULT=mode)
                self.assertEqual(result.returncode, 4)
                self.assertIn("invalid PR checks response", result.stderr)
                self.assertNotIn("release_gate_monitor_pr_checks", result.stdout)

    def test_pr_cli_errors_and_unsupported_exit_codes_do_not_fall_back(self) -> None:
        auth_failure = self.run_monitor("--pr", "123", FAKE_GH_RESULT="auth_failure")
        self.assertEqual(auth_failure.returncode, 4)
        self.assertIn("invalid PR checks response: malformed JSON", auth_failure.stderr)
        self.assertNotIn("unit pass 1s", auth_failure.stdout)

        unsupported_exit = self.run_monitor(
            "--pr", "123", FAKE_GH_RESULT="unsupported_exit"
        )
        self.assertEqual(unsupported_exit.returncode, 1)
        self.assertIn("unable to read PR checks for 123", unsupported_exit.stderr)
        self.assertNotIn("release_gate_monitor_pr_checks", unsupported_exit.stdout)

        for mode in ("partial_auth_failure", "inconsistent_pending_exit"):
            with self.subTest(mode=mode):
                inconsistent = self.run_monitor("--pr", "123", FAKE_GH_RESULT=mode)
                self.assertEqual(inconsistent.returncode, 4)
                self.assertIn("invalid PR checks response", inconsistent.stderr)
                self.assertNotIn("release_gate_monitor_pr_checks", inconsistent.stdout)

    def test_pr_reference_cannot_be_reinterpreted_as_a_gh_option(self) -> None:
        result = self.run_monitor(
            "--pr", "--repo", FAKE_EXPECTED_PR="--repo", FAKE_GH_RESULT="pass"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            self.gh_log.read_text(encoding="utf-8").splitlines(),
            ["pr checks --json name,state,bucket,workflow,link -- --repo"],
        )


if __name__ == "__main__":
    unittest.main()
