# ArchiSyn が生成した Rust ノードの検証用イメージ。
# ROS 2 Humble + Rust toolchain + ros2_rust (rclrs) の underlay ワークスペースを含む。
#
# ビルド:  docker build -t archisyn-humble-rust -f docker/humble-rust.Dockerfile .
# 使い方:  docker run --rm -it -v <生成先>:/ws -w /ws archisyn-humble-rust bash -c \
#            "source /opt/ros2_rust_ws/install/setup.bash && colcon build"
FROM osrf/ros:humble-desktop-full

RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    libclang-dev \
    python3-pip \
    python3-vcstool \
    && rm -rf /var/lib/apt/lists/*

# colcon の cargo 対応プラグイン
RUN pip install --no-cache-dir colcon-cargo colcon-ros-cargo

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

RUN cargo install cargo-ament-build --locked

# ros2_rust を underlay としてビルド
# （rclrs と、common_interfaces 等の Rust メッセージバインディングを生成）
WORKDIR /opt/ros2_rust_ws
RUN mkdir src \
    && git clone https://github.com/ros2-rust/ros2_rust.git src/ros2_rust \
    && vcs import src < src/ros2_rust/ros2_rust_humble.repos \
    && bash -c "source /opt/ros/humble/setup.bash && colcon build"

WORKDIR /ws
