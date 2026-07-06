#!/usr/bin/env python3
"""実行中の ROS 2 システムを解析し、ArchiSyn インポート用の JSON を出力する。

rqt_graph と同じ情報源（ROS graph API）を使うため、言語や実装方法を問わず
起動中の全ノードの Pub/Sub トピック・型・パラメータを取得できる。

使い方（対象システムを起動した ROS 2 環境で）:
    python3 introspect.py -o graph.json
その後、ArchiSyn のメニュー「インポート」から graph.json を読み込む。
"""

import argparse
import json
import time

import rclpy
from rcl_interfaces.srv import GetParameters, ListParameters

EXCLUDED_TOPICS = {"/parameter_events", "/rosout"}
EXCLUDED_NODES = {"archisyn_introspect"}
EXCLUDED_PARAMS_PREFIX = ("qos_overrides",)
EXCLUDED_PARAMS = {"use_sim_time", "start_type_description_service"}

# rcl_interfaces/msg/ParameterType の値 → (ArchiSyn 型, 値の取り出し)
PARAM_TYPES = {
    1: ("bool", lambda v: v.bool_value),
    2: ("int64", lambda v: v.integer_value),
    3: ("float64", lambda v: v.double_value),
    4: ("string", lambda v: v.string_value),
}


def collect_parameters(node, name, ns):
    """他ノードのパラメータをサービス経由で取得（ベストエフォート）"""
    prefix = f"{ns.rstrip('/')}/{name}"
    try:
        list_client = node.create_client(ListParameters, f"{prefix}/list_parameters")
        if not list_client.wait_for_service(timeout_sec=0.5):
            return []
        future = list_client.call_async(ListParameters.Request())
        rclpy.spin_until_future_complete(node, future, timeout_sec=1.0)
        if future.result() is None:
            return []
        names = [
            n
            for n in future.result().result.names
            if n not in EXCLUDED_PARAMS and not n.startswith(EXCLUDED_PARAMS_PREFIX)
        ]
        if not names:
            return []

        get_client = node.create_client(GetParameters, f"{prefix}/get_parameters")
        if not get_client.wait_for_service(timeout_sec=0.5):
            return []
        request = GetParameters.Request()
        request.names = names
        future = get_client.call_async(request)
        rclpy.spin_until_future_complete(node, future, timeout_sec=1.0)
        if future.result() is None:
            return []

        params = []
        for pname, value in zip(names, future.result().values):
            mapped = PARAM_TYPES.get(value.type)
            if mapped:
                ptype, getter = mapped
                params.append({"name": pname, "type": ptype, "value": getter(value)})
        return params
    except Exception:
        return []


def collect_ports(entries):
    ports = []
    for topic, types in entries:
        if topic in EXCLUDED_TOPICS or not types:
            continue
        ports.append({"topic": topic, "type": types[0]})
    return ports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-o", "--output", default="archisyn_graph.json")
    parser.add_argument(
        "--wait", type=float, default=2.0, help="グラフ発見の待ち時間 [s]"
    )
    args = parser.parse_args()

    rclpy.init()
    node = rclpy.create_node("archisyn_introspect")

    # グラフ情報（他ノードの発見）が行き渡るまで待つ
    deadline = time.time() + args.wait
    while time.time() < deadline:
        rclpy.spin_once(node, timeout_sec=0.1)

    nodes = []
    for name, ns in node.get_node_names_and_namespaces():
        if name in EXCLUDED_NODES or name.startswith("_"):
            continue
        nodes.append(
            {
                "name": name,
                "namespace": ns,
                "publishers": collect_ports(
                    node.get_publisher_names_and_types_by_node(name, ns)
                ),
                "subscriptions": collect_ports(
                    node.get_subscriber_names_and_types_by_node(name, ns)
                ),
                "parameters": collect_parameters(node, name, ns),
            }
        )

    data = {"version": "1", "nodes": nodes}
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"{len(nodes)} ノードを {args.output} に書き出しました")

    node.destroy_node()
    rclpy.shutdown()


if __name__ == "__main__":
    main()
