// .arcsyn データモデルの TS 型定義（doc/plan.md §4 の YAML スキーマに対応）

export type Language = "python" | "cpp" | "rust";

export type PortDef = {
  name: string;
  type: string;
};

export type ParamDef = {
  name: string;
  type: string;
  // 編集中は文字列で保持し、コード生成時に型に応じて解釈する
  default: string;
};

export type TypeField = {
  name: string;
  type: string;
};

export type CustomType = {
  name: string;
  fields: TypeField[];
};

export type ArchNodeData = {
  label: string;
  language: Language;
  periodMs: number;
  inputs: PortDef[];
  outputs: PortDef[];
  params: ParamDef[];
};

export const LANGUAGES: readonly Language[] = ["python", "cpp", "rust"];

// ポート型の入力候補（よく使う ROS 2 メッセージ型）
export const ROS_MSG_TYPES: readonly string[] = [
  "std_msgs/Bool",
  "std_msgs/Int32",
  "std_msgs/Int64",
  "std_msgs/Float32",
  "std_msgs/Float64",
  "std_msgs/String",
  "geometry_msgs/Vector3",
  "geometry_msgs/Point",
  "geometry_msgs/Pose",
  "geometry_msgs/Twist",
  "sensor_msgs/Imu",
  "sensor_msgs/LaserScan",
  "sensor_msgs/Image",
  "nav_msgs/Odometry",
];

// カスタム型フィールドに使える基本型（.msg のプリミティブ）
export const PRIMITIVE_TYPES: readonly string[] = [
  "bool",
  "int8",
  "uint8",
  "int16",
  "uint16",
  "int32",
  "uint32",
  "int64",
  "uint64",
  "float32",
  "float64",
  "string",
];

// ROS 2 パラメータとして扱える型
export const PARAM_TYPES: readonly string[] = [
  "bool",
  "int64",
  "float64",
  "string",
];

// 型互換チェック（F-1.1 最小仕様: 完全一致）
export function isTypeCompatible(sourceType: string, targetType: string) {
  return sourceType === targetType;
}
