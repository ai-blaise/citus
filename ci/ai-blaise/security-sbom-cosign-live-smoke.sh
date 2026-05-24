#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Sec9
# Live SBOM/cosign proof for release attestation plumbing. This smoke uses a
# local OCI registry, a digest-pinned ai-blaise Citus image, Syft SPDX output,
# Cosign image signatures, SPDX/SLSA attestations, and blob bundle verification.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

python3 <<'PY_SMOKE'
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time

DEFAULT_SOURCE_IMAGE = "ai-blaise-citus-timescale-cohabitation:local"
SOURCE_IMAGE = os.environ.get("SEC9_SOURCE_IMAGE", DEFAULT_SOURCE_IMAGE)
REGISTRY_IMAGE = os.environ.get("SEC9_REGISTRY_IMAGE", "registry:2")
SYFT_IMAGE = os.environ.get("SEC9_SYFT_IMAGE", "ghcr.io/anchore/syft:v1.18.1")
COSIGN_IMAGE = os.environ.get("SEC9_COSIGN_IMAGE", "gcr.io/projectsigstore/cosign:v2.4.1")
COHAB_DOCKERFILE = Path("images/citus-timescale-cohabitation/Dockerfile")
ARTIFACT = Path("artifacts/sec9-sbom-cosign-live-evidence.tsv")

containers = []


def fail(message):
    raise AssertionError(message)


def require_tool(name):
    if shutil.which(name) is None:
        fail(f"{name} is required for Sec9 SBOM/cosign live smoke")


def run(args, *, input_text=None, env=None, timeout=300, check=True):
    result = subprocess.run(
        args,
        input=input_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        timeout=timeout,
    )
    if check and result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        fail(f"command failed with exit {result.returncode}: {' '.join(args)}")
    return result


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def docker_image_exists(image):
    return run(["docker", "image", "inspect", image], check=False, timeout=30).returncode == 0


def ensure_source_image():
    if docker_image_exists(SOURCE_IMAGE):
        return "existing"
    if SOURCE_IMAGE != DEFAULT_SOURCE_IMAGE:
        fail(f"SEC9 source image is missing: {SOURCE_IMAGE}")
    if not COHAB_DOCKERFILE.exists():
        fail(f"missing source-image Dockerfile: {COHAB_DOCKERFILE}")
    run([
        "docker", "build",
        "--file", str(COHAB_DOCKERFILE),
        "--build-arg", os.environ.get("TIMESCALE_COHABITATION_BASE_ARG", "BASE_IMAGE=timescale/timescaledb-ha:pg17-ts2.27"),
        "--build-arg", os.environ.get("TIMESCALE_COHABITATION_MAKE_ARG", "MAKE_JOBS=4"),
        "--tag", SOURCE_IMAGE,
        os.getcwd(),
    ], timeout=1800)
    return "built"


def start_registry():
    port = free_port()
    name = f"ai-blaise-sec9-registry-{os.getpid()}-{int(time.time())}"
    run(["docker", "pull", REGISTRY_IMAGE], timeout=300)
    run(["docker", "run", "-d", "--name", name, "-p", f"127.0.0.1:{port}:5000", REGISTRY_IMAGE], timeout=120)
    containers.append(name)
    registry = f"127.0.0.1:{port}"
    for _ in range(80):
        result = run(["docker", "logs", name], check=False, timeout=10)
        if "listening on" in (result.stdout + result.stderr).lower():
            return registry
        time.sleep(0.25)
    return registry


def cleanup():
    for container in containers:
        run(["docker", "rm", "-f", container], check=False, timeout=60)


def push_digest_pinned_image(registry):
    tag_ref = f"{registry}/ai-blaise/citus-sec9:live-smoke"
    run(["docker", "tag", SOURCE_IMAGE, tag_ref], timeout=120)
    push = run(["docker", "push", tag_ref], timeout=600)
    combined = push.stdout + push.stderr
    match = re.search(r"digest: (sha256:[0-9a-f]{64})", combined)
    if not match:
        fail(f"could not parse pushed digest from docker push output:\n{combined}")
    digest = match.group(1)
    return tag_ref, f"{registry}/ai-blaise/citus-sec9@{digest}", digest


def validate_spdx(path):
    data = json.loads(path.read_text())
    if data.get("spdxVersion") != "SPDX-2.3":
        fail(f"unexpected SPDX version: {data.get('spdxVersion')}")
    packages = data.get("packages") or []
    if not packages:
        fail("SPDX SBOM did not contain packages")
    if not data.get("documentNamespace"):
        fail("SPDX SBOM lost documentNamespace")
    return len(packages)


def cosign_base(tmpdir):
    uid_gid = f"{os.getuid()}:{os.getgid()}"
    return [
        "docker", "run", "--rm", "--network", "host", "--user", uid_gid,
        "-v", f"{tmpdir}:/work", COSIGN_IMAGE,
    ]


def cosign(tmpdir, args, *, env=None, timeout=300):
    command = cosign_base(tmpdir)
    if env:
        for key, value in env.items():
            command[2:2] = ["-e", f"{key}={value}"]
    command.extend(args)
    return run(command, timeout=timeout)


def generate_sbom(tmpdir):
    sbom = Path(tmpdir) / "sec9.spdx.json"
    run([
        "docker", "run", "--rm",
        "-v", "/var/run/docker.sock:/var/run/docker.sock",
        "-v", f"{tmpdir}:/work",
        SYFT_IMAGE,
        f"docker:{SOURCE_IMAGE}",
        "-o", "spdx-json=/work/sec9.spdx.json",
    ], timeout=900)
    package_count = validate_spdx(sbom)
    return sbom, package_count


def write_slsa_predicate(tmpdir, digest_ref, digest):
    predicate = Path(tmpdir) / "sec9.slsa-provenance.json"
    predicate.write_text(json.dumps({
        "_type": "https://slsa.dev/provenance/v1",
        "subject": [{
            "name": digest_ref,
            "digest": {"sha256": digest.removeprefix("sha256:")},
        }],
        "buildDefinition": {
            "buildType": "https://ai-blaise.local/sec9/live-smoke",
            "externalParameters": {
                "sourceImage": SOURCE_IMAGE,
                "syftImage": SYFT_IMAGE,
                "cosignImage": COSIGN_IMAGE,
            },
            "internalParameters": {},
            "resolvedDependencies": [],
        },
        "runDetails": {
            "builder": {"id": "experiment-playground-vm"},
            "metadata": {"invocationId": f"sec9-{os.getpid()}"},
        },
    }, sort_keys=True))
    return predicate


def parse_cosign_json(path):
    data = json.loads(path.read_text())
    if isinstance(data, list):
        if not data:
            fail(f"cosign JSON output was empty: {path}")
        return data
    if isinstance(data, dict):
        if not data:
            fail(f"cosign JSON object was empty: {path}")
        return data
    fail(f"unexpected cosign JSON shape in {path}: {type(data).__name__}")


def main():
    require_tool("docker")
    source_state = ensure_source_image()
    with tempfile.TemporaryDirectory(prefix="ai-blaise-sec9-") as tmp:
        os.chmod(tmp, 0o777)
        registry = start_registry()
        _tag_ref, digest_ref, digest = push_digest_pinned_image(registry)
        sbom, package_count = generate_sbom(tmp)
        slsa_predicate = write_slsa_predicate(tmp, digest_ref, digest)

        cosign(tmp, ["generate-key-pair", "--output-key-prefix", "/work/cosign"], env={"COSIGN_PASSWORD": ""})

        cosign(tmp, [
            "sign", "--key", "/work/cosign.key",
            "--allow-http-registry", "--allow-insecure-registry",
            "--tlog-upload=false", "--yes",
            "-a", "slsa.dev/provenance/v1=sec9-live-smoke",
            digest_ref,
        ], env={"COSIGN_PASSWORD": ""})
        verify_image = Path(tmp) / "sec9-image-verify.json"
        image_verify = cosign(tmp, [
            "verify", "--key", "/work/cosign.pub",
            "--allow-http-registry", "--allow-insecure-registry", "--insecure-ignore-tlog",
            "-a", "slsa.dev/provenance/v1=sec9-live-smoke",
            digest_ref,
        ])
        verify_image.write_text(image_verify.stdout)
        image_verify_json = parse_cosign_json(verify_image)
        if isinstance(image_verify_json, list):
            signed_digest = image_verify_json[0]["critical"]["image"]["docker-manifest-digest"]
            if signed_digest != digest:
                fail(f"verified signature digest mismatch: {signed_digest} != {digest}")

        for predicate_path, predicate_type in ((sbom, "spdxjson"), (slsa_predicate, "slsaprovenance1")):
            cosign(tmp, [
                "attest", "--predicate", f"/work/{predicate_path.name}", "--type", predicate_type,
                "--key", "/work/cosign.key",
                "--allow-http-registry", "--allow-insecure-registry",
                "--tlog-upload=false", "--yes",
                digest_ref,
            ], env={"COSIGN_PASSWORD": ""}, timeout=600)
            verify_attestation = cosign(tmp, [
                "verify-attestation", "--type", predicate_type,
                "--key", "/work/cosign.pub",
                "--allow-http-registry", "--allow-insecure-registry", "--insecure-ignore-tlog",
                digest_ref,
            ], timeout=600)
            attestation_json = Path(tmp) / f"sec9-{predicate_type}-verify.json"
            attestation_json.write_text(verify_attestation.stdout)
            parse_cosign_json(attestation_json)

        bundle = Path(tmp) / "sec9.spdx.sigstore.json"
        signature = Path(tmp) / "sec9.spdx.sig"
        cosign(tmp, [
            "sign-blob", "--key", "/work/cosign.key",
            "--bundle", f"/work/{bundle.name}",
            "--output-signature", f"/work/{signature.name}",
            "--tlog-upload=false", "--yes", f"/work/{sbom.name}",
        ], env={"COSIGN_PASSWORD": ""})
        bundle_json = json.loads(bundle.read_text())
        if "base64Signature" not in bundle_json:
            fail("sigstore bundle lost base64Signature")
        cosign(tmp, [
            "verify-blob", "--key", "/work/cosign.pub",
            "--signature", f"/work/{signature.name}",
            "--bundle", f"/work/{bundle.name}",
            "--insecure-ignore-tlog", f"/work/{sbom.name}",
        ])

        ARTIFACT.parent.mkdir(parents=True, exist_ok=True)
        ARTIFACT.write_text(
            "feature\tassertion\tstatus\tdetail\n"
            f"Sec9\tsource_image\tpassed\t{SOURCE_IMAGE} ({source_state})\n"
            f"Sec9\tregistry_digest\tpassed\t{digest_ref}\n"
            f"Sec9\tspdx_sbom\tpassed\tpackages={package_count} artifact=sec9.spdx.json\n"
            "Sec9\tcosign_image_signature\tpassed\tverify --key cosign.pub matched digest and annotation\n"
            "Sec9\tspdx_attestation\tpassed\tcosign attest/verify-attestation --type spdxjson\n"
            "Sec9\tslsa_attestation\tpassed\tcosign attest/verify-attestation --type slsaprovenance1\n"
            "Sec9\tsigstore_bundle\tpassed\tsec9.spdx.sigstore.json verify-blob OK\n"
        )
    print("ai_blaise_citus Sec9 SBOM/cosign live smoke passed")


try:
    main()
finally:
    cleanup()
PY_SMOKE
