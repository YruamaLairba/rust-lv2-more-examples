cargo build -p eg-worker-rs
mkdir -p test/eg-worker-rs/
cp eg-worker-rs/worker.ttl test/eg-worker-rs/
cp eg-worker-rs/manifest.ttl test/eg-worker-rs/
cp target/debug/libeg_worker_rs.so test/eg-worker-rs/
LV2_PATH=$PWD/test/ jalv urn:rust-lv2-more-examples:eg-worker-rs
