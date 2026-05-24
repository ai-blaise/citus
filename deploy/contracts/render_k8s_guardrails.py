#!/usr/bin/env python3
"""Render the ai-blaise Kubernetes production guardrail contract.

The Helm chart lives in ai-blaise/command-center after the 2026-05-22 chart
fold. This renderer keeps the Citus-side contract for the app labels, target
names, and guardrail resources that the chart must continue to honor.
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys
from typing import Any

APP_NAME = "ai-blaise-citus"
CONTRACT_LABEL = "k8s-production-guardrails"
HTTP_PORT = 8080
POOL_PORT = 5432

SIDECARS = (
    ("analytical", "io-heavy", "db", 2),
    ("auth", "stateless", "api", 2),
    ("backup", "io-heavy", "db", 2),
    ("cdc", "io-heavy", "db", 2),
    ("coldtier", "db-internal", "db", 2),
    ("edge-functions", "stateless", "api", 2),
    ("graphql", "stateless", "api", 2),
    ("hlc", "consensus", "consensus", 3),
    ("mcp", "stateless", "api", 2),
    ("postgrest", "stateless", "api", 2),
    ("raft", "consensus", "consensus", 3),
    ("realtime", "io-heavy", "db", 2),
    ("repack", "db-internal", "db", 2),
    ("schema-job", "db-internal", "db", 2),
    ("storage", "io-heavy", "db", 2),
    ("txn-status", "consensus", "consensus", 3),
    ("vectorizer", "io-heavy", "db", 2),
)

HPA_SIDECARS = {
    "analytical",
    "auth",
    "cdc",
    "edge-functions",
    "graphql",
    "mcp",
    "postgrest",
    "realtime",
    "storage",
    "vectorizer",
}
CUSTOM_QUEUE_HPA_SIDECARS = {"cdc", "realtime", "vectorizer"}
CONSENSUS_SIDECARS = {name for name, tier, _, _ in SIDECARS if tier == "consensus"}
PRIVATE_POOL_CIDRS = ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16")


def labels(component: str, extra: dict[str, str] | None = None) -> dict[str, str]:
    base = {
        "app.kubernetes.io/name": APP_NAME,
        "app.kubernetes.io/component": component,
        "app.kubernetes.io/managed-by": "ai-blaise-citus-contract",
        "ai-blaise.com/deployment-contract": CONTRACT_LABEL,
    }
    if extra:
        base.update(extra)
    return base


def metadata(
    name: str, component: str, extra: dict[str, str] | None = None
) -> dict[str, Any]:
    return {
        "name": name,
        "labels": labels(component, extra),
        "annotations": {
            "ai-blaise.com/chart": "command-center/citus-cluster",
            "ai-blaise.com/chart-fold-date": "2026-05-22",
        },
    }


def pod_selector(component: str) -> dict[str, Any]:
    return {
        "matchLabels": {
            "app.kubernetes.io/name": APP_NAME,
            "app.kubernetes.io/component": component,
        }
    }


def pod_peer(component: str) -> dict[str, Any]:
    return {"podSelector": pod_selector(component)}


def prometheus_peer() -> dict[str, Any]:
    return {
        "namespaceSelector": {},
        "podSelector": {"matchLabels": {"app.kubernetes.io/name": "prometheus"}},
    }


def client_namespace_peer() -> dict[str, Any]:
    return {"namespaceSelector": {"matchLabels": {"ai-blaise.com/pool-client": "true"}}}


def cnpg_peer() -> dict[str, Any]:
    return {"podSelector": {"matchLabels": {"cnpg.io/cluster": f"{APP_NAME}-citus"}}}


def port(port_number: int) -> dict[str, Any]:
    return {"protocol": "TCP", "port": port_number}


def pdb(name: str, component: str, tier: str, replicas: int) -> dict[str, Any]:
    spec: dict[str, Any] = {"selector": pod_selector(component)}
    if component == "operator":
        spec["minAvailable"] = 1
    elif component == "pool":
        spec["minAvailable"] = 2
    elif tier == "consensus":
        spec["minAvailable"] = max(2, replicas - 1)
    else:
        spec["maxUnavailable"] = "33%"

    return {
        "apiVersion": "policy/v1",
        "kind": "PodDisruptionBudget",
        "metadata": metadata(name, component, {"ai-blaise.com/guardrail": "pdb"}),
        "spec": spec,
    }


def hpa(
    name: str,
    component: str,
    min_replicas: int,
    max_replicas: int,
    custom_queue: bool,
) -> dict[str, Any]:
    metrics: list[dict[str, Any]] = [
        {
            "type": "Resource",
            "resource": {
                "name": "cpu",
                "target": {"type": "Utilization", "averageUtilization": 70},
            },
        },
        {
            "type": "Resource",
            "resource": {
                "name": "memory",
                "target": {"type": "Utilization", "averageUtilization": 80},
            },
        },
    ]
    if custom_queue:
        metrics.append(
            {
                "type": "Pods",
                "pods": {
                    "metric": {"name": "ai_blaise_sidecar_queue_depth"},
                    "target": {"type": "AverageValue", "averageValue": "100"},
                },
            }
        )

    return {
        "apiVersion": "autoscaling/v2",
        "kind": "HorizontalPodAutoscaler",
        "metadata": metadata(name, component, {"ai-blaise.com/guardrail": "hpa"}),
        "spec": {
            "scaleTargetRef": {
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "name": name,
            },
            "minReplicas": min_replicas,
            "maxReplicas": max_replicas,
            "behavior": {
                "scaleDown": {
                    "stabilizationWindowSeconds": 300,
                    "policies": [{"type": "Percent", "value": 50, "periodSeconds": 60}],
                },
                "scaleUp": {
                    "stabilizationWindowSeconds": 0,
                    "policies": [
                        {"type": "Percent", "value": 100, "periodSeconds": 30},
                        {"type": "Pods", "value": 2, "periodSeconds": 30},
                    ],
                    "selectPolicy": "Max",
                },
            },
            "metrics": metrics,
        },
    }


def network_policy(
    name: str, component: str, ingress: list[dict[str, Any]]
) -> dict[str, Any]:
    return {
        "apiVersion": "networking.k8s.io/v1",
        "kind": "NetworkPolicy",
        "metadata": metadata(
            name, component, {"ai-blaise.com/guardrail": "network-policy"}
        ),
        "spec": {
            "podSelector": pod_selector(component),
            "policyTypes": ["Ingress"],
            "ingress": ingress,
        },
    }


def operator_network_policy() -> dict[str, Any]:
    return network_policy(
        f"{APP_NAME}-operator",
        "operator",
        [{"from": [prometheus_peer()], "ports": [port(HTTP_PORT)]}],
    )


def pool_network_policy() -> dict[str, Any]:
    postgres_peers = [client_namespace_peer()]
    postgres_peers.extend({"ipBlock": {"cidr": cidr}} for cidr in PRIVATE_POOL_CIDRS)
    return network_policy(
        f"{APP_NAME}-pool-postgres",
        "pool",
        [
            {
                "from": [pod_peer("operator"), prometheus_peer()],
                "ports": [port(HTTP_PORT)],
            },
            {"from": postgres_peers, "ports": [port(POOL_PORT)]},
        ],
    )


def sidecar_network_policy(name: str, network_tier: str) -> dict[str, Any]:
    component = f"sidecar-{name}"
    if network_tier == "api":
        peers = [pod_peer("pool"), pod_peer("operator")]
    elif network_tier == "db":
        peers = [pod_peer("operator"), pod_peer("pool"), cnpg_peer()]
    elif network_tier == "consensus":
        peers = [pod_peer(component), pod_peer("operator")]
    else:
        raise ValueError(f"unknown network tier for {name}: {network_tier}")
    peers.append(prometheus_peer())
    return network_policy(
        f"{APP_NAME}-sidecar-{name}",
        component,
        [{"from": peers, "ports": [port(HTTP_PORT)]}],
    )


def build_resources() -> list[dict[str, Any]]:
    resources: list[dict[str, Any]] = []

    resources.append(pdb(f"{APP_NAME}-operator", "operator", "control-plane", 2))
    resources.append(pdb(f"{APP_NAME}-pool", "pool", "data-plane", 3))
    resources.append(hpa(f"{APP_NAME}-pool", "pool", 3, 12, False))
    resources.append(operator_network_policy())
    resources.append(pool_network_policy())

    for name, tier, network_tier, replicas in SIDECARS:
        component = f"sidecar-{name}"
        resource_name = f"{APP_NAME}-sidecar-{name}"
        resources.append(pdb(resource_name, component, tier, replicas))
        if name in HPA_SIDECARS:
            resources.append(
                hpa(resource_name, component, 2, 10, name in CUSTOM_QUEUE_HPA_SIDECARS)
            )
        resources.append(sidecar_network_policy(name, network_tier))

    return resources


def yaml_scalar(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if value is None:
        return "null"
    return json.dumps(str(value))


def dump_yaml(value: Any, indent: int = 0) -> list[str]:
    pad = " " * indent
    if isinstance(value, dict):
        lines: list[str] = []
        for key, child in value.items():
            if isinstance(child, dict) and not child:
                lines.append(f"{pad}{key}: {{}}")
            elif isinstance(child, list) and not child:
                lines.append(f"{pad}{key}: []")
            elif isinstance(child, (dict, list)):
                lines.append(f"{pad}{key}:")
                lines.extend(dump_yaml(child, indent + 2))
            else:
                lines.append(f"{pad}{key}: {yaml_scalar(child)}")
        return lines
    if isinstance(value, list):
        lines = []
        for item in value:
            if isinstance(item, dict) and not item:
                lines.append(f"{pad}- {{}}")
            elif isinstance(item, list) and not item:
                lines.append(f"{pad}- []")
            elif isinstance(item, (dict, list)):
                lines.append(f"{pad}-")
                lines.extend(dump_yaml(item, indent + 2))
            else:
                lines.append(f"{pad}- {yaml_scalar(item)}")
        return lines
    return [f"{pad}{yaml_scalar(value)}"]


def render(resources: list[dict[str, Any]]) -> str:
    docs = ["\n".join(dump_yaml(resource)) for resource in resources]
    return "---\n" + "\n---\n".join(docs) + "\n"


def validate(resources: list[dict[str, Any]]) -> str:
    by_kind = collections.Counter(resource["kind"] for resource in resources)
    names = [(resource["kind"], resource["metadata"]["name"]) for resource in resources]
    duplicates = [
        name for name, count in collections.Counter(names).items() if count > 1
    ]
    if duplicates:
        raise AssertionError(f"duplicate guardrail resources: {duplicates}")

    expected_pdbs = 2 + len(SIDECARS)
    expected_hpas = 1 + len(HPA_SIDECARS)
    expected_network_policies = 2 + len(SIDECARS)
    if by_kind["PodDisruptionBudget"] != expected_pdbs:
        raise AssertionError(
            f"PDB count mismatch: {by_kind['PodDisruptionBudget']} != {expected_pdbs}"
        )
    if by_kind["HorizontalPodAutoscaler"] != expected_hpas:
        raise AssertionError(
            f"HPA count mismatch: {by_kind['HorizontalPodAutoscaler']} != {expected_hpas}"
        )
    if by_kind["NetworkPolicy"] != expected_network_policies:
        raise AssertionError(
            f"NetworkPolicy count mismatch: {by_kind['NetworkPolicy']} != {expected_network_policies}"
        )

    hpa_targets = {
        resource["spec"]["scaleTargetRef"]["name"]
        for resource in resources
        if resource["kind"] == "HorizontalPodAutoscaler"
    }
    for consensus in CONSENSUS_SIDECARS:
        if f"{APP_NAME}-sidecar-{consensus}" in hpa_targets:
            raise AssertionError(f"consensus sidecar must not autoscale: {consensus}")
    for sidecar in HPA_SIDECARS:
        if f"{APP_NAME}-sidecar-{sidecar}" not in hpa_targets:
            raise AssertionError(f"missing HPA for sidecar: {sidecar}")
    if f"{APP_NAME}-pool" not in hpa_targets:
        raise AssertionError("missing HPA for pool")

    for resource in resources:
        component = resource["metadata"]["labels"]["app.kubernetes.io/component"]
        if resource["kind"] == "NetworkPolicy":
            spec = resource["spec"]
            if spec["policyTypes"] != ["Ingress"]:
                raise AssertionError(
                    f"{resource['metadata']['name']} must be ingress-only"
                )
            if spec["podSelector"] != pod_selector(component):
                raise AssertionError(
                    f"selector mismatch for {resource['metadata']['name']}"
                )
        if resource["kind"] == "HorizontalPodAutoscaler":
            metrics = resource["spec"].get("metrics", [])
            metric_names = {metric["type"] for metric in metrics}
            if not {"Resource"}.issubset(metric_names):
                raise AssertionError(
                    f"resource metrics missing for {resource['metadata']['name']}"
                )
            if resource["spec"]["maxReplicas"] <= resource["spec"]["minReplicas"]:
                raise AssertionError(
                    f"invalid replica range for {resource['metadata']['name']}"
                )
            if "behavior" not in resource["spec"]:
                raise AssertionError(
                    f"missing HPA behavior for {resource['metadata']['name']}"
                )

    pool_policy = next(
        resource
        for resource in resources
        if resource["kind"] == "NetworkPolicy"
        and resource["metadata"]["name"] == f"{APP_NAME}-pool-postgres"
    )
    pool_ingress = pool_policy["spec"]["ingress"]
    cidrs = {
        peer["ipBlock"]["cidr"]
        for rule in pool_ingress
        for peer in rule.get("from", [])
        if "ipBlock" in peer
    }
    if cidrs != set(PRIVATE_POOL_CIDRS):
        raise AssertionError(f"pool CIDR allowlist mismatch: {sorted(cidrs)}")

    return (
        "k8s_guardrails_contract\t"
        f"resources={len(resources)}\t"
        f"hpa={by_kind['HorizontalPodAutoscaler']}\t"
        f"pdb={by_kind['PodDisruptionBudget']}\t"
        f"network_policy={by_kind['NetworkPolicy']}\t"
        "chart_folded_to_command_center=2026-05-22"
    )


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--check-file", type=pathlib.Path)
    args = parser.parse_args(argv)

    resources = build_resources()
    if args.validate_only:
        print(validate(resources))
        return 0

    rendered = render(resources)
    if args.check_file is not None:
        expected = args.check_file.read_text(encoding="utf-8")
        if expected != rendered:
            print(
                f"{args.check_file} is not in sync with rendered guardrails",
                file=sys.stderr,
            )
            return 1
    sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
