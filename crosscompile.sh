# cargo build --release \
#     --target x86_64-unknown-linux-gnu \
#     --target x86_64-pc-windows-gnu 

TURUNMAP_VERSION=$(cat Cargo.toml | rg "^version = \".*?\"$" | rg -o "\\d+\\.\\d+\\.\\d+")

echo "Building for Version $TURUNMAP_VERSION"
cargo build --release --target x86_64-unknown-linux-gnu 

echo "Moving to tar ball"
tar -czf "TurunMap-V$TURUNMAP_VERSION-x86_64-unknown-linux-gnu.tar.gz" "target/x86_64-unknown-linux-gnu/release/turunmap"

echo "Done! Version $TURUNMAP_VERSION is ready for upload!"
