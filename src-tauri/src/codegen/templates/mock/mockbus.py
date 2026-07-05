# 自動生成ファイル — ArchiSyn (mock_pubsub) の極小 Pub/Sub バスとノード基底です。
# コード生成のたびに上書きされるため、編集しないでください。
import threading
import time


class Bus:
    """トピック名 → 購読コールバックへ同期ディスパッチする極小バス"""

    def __init__(self) -> None:
        self._subs: dict[str, list] = {}
        self._lock = threading.Lock()

    def subscribe(self, topic: str, callback) -> None:
        with self._lock:
            self._subs.setdefault(topic, []).append(callback)

    def publish(self, topic: str, msg) -> None:
        with self._lock:
            callbacks = list(self._subs.get(topic, []))
        for callback in callbacks:
            callback(msg)


class MockNode:
    """period_ms ごとに on_update() を呼ぶノード基底"""

    def __init__(self, name: str, period_ms: int, bus: Bus) -> None:
        self.name = name
        self.period_s = period_ms / 1000.0
        self.bus = bus
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._loop, name=name, daemon=True)

    def start(self) -> None:
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()

    def log(self, message: str) -> None:
        print(f"[{self.name}] {message}", flush=True)

    def _loop(self) -> None:
        while not self._stop.is_set():
            self.on_update()
            time.sleep(self.period_s)

    def on_update(self) -> None:
        raise NotImplementedError
