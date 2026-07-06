import { describe, expect, it } from "vitest";
import { parseTypeTable, typesToTsv } from "./tableParse";

describe("parseTypeTable", () => {
  it("3列 TSV（Excel コピー形式）で複数型を一括定義できる", () => {
    const text = [
      "型名\tフィールド名\t型",
      "FusedPose\tposition\tgeometry_msgs/Vector3",
      "\tconfidence\tfloat64",
      "WheelCmd\tleft\tfloat64",
      "WheelCmd\tright\tfloat64",
    ].join("\n");
    const r = parseTypeTable(text);
    expect(r.fieldsOnly).toBe(false);
    expect(r.types).toEqual([
      {
        name: "FusedPose",
        fields: [
          { name: "position", type: "geometry_msgs/Vector3" },
          { name: "confidence", type: "float64" },
        ],
      },
      {
        name: "WheelCmd",
        fields: [
          { name: "left", type: "float64" },
          { name: "right", type: "float64" },
        ],
      },
    ]);
    expect(r.warnings).toEqual([]);
  });

  it("2列はフィールドのみモードになる", () => {
    const r = parseTypeTable("x\tfloat64\ny\tfloat64");
    expect(r.fieldsOnly).toBe(true);
    expect(r.types).toHaveLength(1);
    expect(r.types[0].fields).toEqual([
      { name: "x", type: "float64" },
      { name: "y", type: "float64" },
    ]);
  });

  it("CSV（カンマ区切り）も受け付ける", () => {
    const r = parseTypeTable("Pose,x,float64\nPose,y,float64");
    expect(r.types[0].name).toBe("Pose");
    expect(r.types[0].fields).toHaveLength(2);
  });

  it("ヘッダ行（英語）を自動スキップする", () => {
    const r = parseTypeTable("name,type\nx,float64");
    expect(r.fieldsOnly).toBe(true);
    expect(r.types[0].fields).toEqual([{ name: "x", type: "float64" }]);
  });

  it("不正な行は警告してスキップする", () => {
    const text = [
      "Pose\tx\tfloat64",
      "Pose\t1abc\tfloat64", // 不正なフィールド名
      "Pose\ty\tbad type!", // 不正な型
      "Pose\tx\tfloat64", // 重複フィールド
    ].join("\n");
    const r = parseTypeTable(text);
    expect(r.types[0].fields).toEqual([{ name: "x", type: "float64" }]);
    expect(r.warnings).toHaveLength(3);
    expect(r.warnings[0]).toContain("1abc");
    expect(r.warnings[1]).toContain("bad type!");
    expect(r.warnings[2]).toContain("重複");
  });

  it("空入力は警告を返す", () => {
    const r = parseTypeTable("  \n ");
    expect(r.types).toEqual([]);
    expect(r.warnings.length).toBeGreaterThan(0);
  });

  it("ダブルクォート付き CSV セルを剥がす", () => {
    const r = parseTypeTable('"Pose","x","float64"');
    expect(r.types[0]).toEqual({
      name: "Pose",
      fields: [{ name: "x", type: "float64" }],
    });
  });
});

describe("typesToTsv", () => {
  it("型定義を TSV に変換し、roundtrip できる", () => {
    const types = [
      {
        name: "FusedPose",
        fields: [
          { name: "position", type: "geometry_msgs/Vector3" },
          { name: "confidence", type: "float64" },
        ],
      },
    ];
    const tsv = typesToTsv(types);
    expect(tsv).toContain("FusedPose\tposition\tgeometry_msgs/Vector3");
    const back = parseTypeTable(tsv);
    expect(back.types).toEqual(types);
    expect(back.warnings).toEqual([]);
  });
});
