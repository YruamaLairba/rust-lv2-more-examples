cargo build -p worker
mkdir -p test/worker/
cp worker/worker.ttl test/worker/
cp worker/manifest.ttl test/worker/
cp target/debug/libworker.so test/worker/
LV2_PATH=$PWD/test/ jalv urn:rust-lv2-more-examples:eg-worker-rs
