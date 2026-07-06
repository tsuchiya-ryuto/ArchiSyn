import { describe, expect, it } from "vitest";
import { searchTypes, type TypeCandidate } from "./typeSearch";

const candidates: TypeCandidate[] = [
  { value: "sensor_msgs/Imu", custom: false },
  { value: "sensor_msgs/Image", custom: false },
  { value: "sensor_msgs/CompressedImage", custom: false },
  { value: "geometry_msgs/Pose", custom: false },
  { value: "geometry_msgs/PoseStamped", custom: false },
  { value: "FusedPose", custom: true },
];

describe("searchTypes", () => {
  it("型名の前方一致が最上位に来る", () => {
    const r = searchTypes("im", candidates).map((c) => c.value);
    // 前方一致: Image, Imu / 部分一致: CompressedImage
    expect(r[0]).toBe("sensor_msgs/Image");
    expect(r[1]).toBe("sensor_msgs/Imu");
    expect(r).toContain("sensor_msgs/CompressedImage");
  });

  it("前方一致が部分一致（カスタム型含む）より優先される", () => {
    const r = searchTypes("pose", candidates).map((c) => c.value);
    expect(r).toEqual([
      "geometry_msgs/Pose", // 前方一致
      "geometry_msgs/PoseStamped", // 前方一致
      "FusedPose", // 部分一致
    ]);
  });

  it("同ランクではカスタム型が先頭に来る", () => {
    const r = searchTypes("fused", [
      { value: "other_msgs/FusedData", custom: false },
      { value: "FusedPose", custom: true },
    ]).map((c) => c.value);
    expect(r[0]).toBe("FusedPose");
  });

  it("パッケージ名でも検索できる", () => {
    const r = searchTypes("geometry", candidates).map((c) => c.value);
    expect(r).toEqual(["geometry_msgs/Pose", "geometry_msgs/PoseStamped"]);
  });

  it("空クエリは全件（limit まで）", () => {
    expect(searchTypes("", candidates)).toHaveLength(candidates.length);
  });

  it("大文字小文字を無視する", () => {
    const r = searchTypes("IMU", candidates).map((c) => c.value);
    expect(r[0]).toBe("sensor_msgs/Imu");
  });

  it("limit で件数を制限する", () => {
    expect(searchTypes("", candidates, 3)).toHaveLength(3);
  });
});
