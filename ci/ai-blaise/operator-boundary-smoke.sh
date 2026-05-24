#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

expected=$'controller	resource	mode	render_plan	kubernetes_apply	direct_sql	status_mutation	conditions	retry_class	requeue_seconds
CitusCluster	ai-blaise-citus	dry-run	1	1	0	1	SpecAccepted=True:Validated,PlanRendered=True:Rendered,DryRun=True:NoMutations,KubernetesApplyAlpha=False:AlphaNotImplemented,StatusMutationAlpha=False:AlphaNotImplemented	alpha-blocked	30
Hypertable	metrics	dry-run	1	0	1	1	SpecAccepted=True:Validated,PlanRendered=True:Rendered,DryRun=True:NoMutations,DirectSqlAlpha=False:AlphaNotImplemented,StatusMutationAlpha=False:AlphaNotImplemented	alpha-blocked	30
Migration	users-display-name	dry-run	1	1	0	1	SpecAccepted=True:Validated,PlanRendered=True:Rendered,DryRun=True:NoMutations,KubernetesApplyAlpha=False:AlphaNotImplemented,StatusMutationAlpha=False:AlphaNotImplemented	alpha-blocked	30
Tenant	tenant-a	dry-run	1	0	1	1	SpecAccepted=True:Validated,PlanRendered=True:Rendered,DryRun=True:NoMutations,DirectSqlAlpha=False:AlphaNotImplemented,StatusMutationAlpha=False:AlphaNotImplemented	alpha-blocked	30'
output="$(cargo run -q -p ai_blaise_citus_operator -- run-controller-boundary)"

if [[ "${output}" != "${expected}" ]]; then
  echo "operator controller boundary report did not match expected dry-run contract" >&2
  echo "Expected:" >&2
  printf '%s
' "${expected}" >&2
  echo "Actual:" >&2
  printf '%s
' "${output}" >&2
  exit 1
fi

if AI_BLAISE_OPERATOR_EXECUTION_MODE=apply cargo run -q -p ai_blaise_citus_operator -- run-controller-boundary >/tmp/operator-boundary-apply.out 2>/tmp/operator-boundary-apply.err; then
  cat /tmp/operator-boundary-apply.out >&2
  cat /tmp/operator-boundary-apply.err >&2
  echo "operator apply mode unexpectedly rendered alpha mutating operations" >&2
  exit 1
fi

if ! grep -Fq "apply mode blocked" /tmp/operator-boundary-apply.err; then
  cat /tmp/operator-boundary-apply.out >&2
  cat /tmp/operator-boundary-apply.err >&2
  echo "operator apply mode did not surface the guarded boundary failure" >&2
  exit 1
fi

rm -f /tmp/operator-boundary-apply.out /tmp/operator-boundary-apply.err

echo "ai_blaise_citus_operator boundary smoke passed"
