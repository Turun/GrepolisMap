
# cargo build --release \
#     --target x86_64-unknown-linux-gnu \
#     --target x86_64-pc-windows-gnu 

# TODO use cargo bundle to generate the applications instead. 
#  https://github.com/burtonageo/cargo-bundle

TURUNMAP_VERSION=$(cat Cargo.toml | rg "^version = \".*?\"$" | rg -o "\\d+\\.\\d+\\.\\d+")

echo "Building for Version $TURUNMAP_VERSION"
cargo build --release --target x86_64-unknown-linux-gnu 
cargo build --release --target x86_64-pc-windows-gnu 
cargo build --release --target x86_64-pc-windows-msvc 
cargo build --release --target aarch64-apple-darwin

echo "Moving to tar ball"
tar -czf "TurunMap-V$TURUNMAP_VERSION-x86_64-unknown-linux-gnu.tar.gz" "target/x86_64-unknown-linux-gnu/release/turunmap"
tar -czf "TurunMap-V$TURUNMAP_VERSION-aarch64-apple-darwin.tar.gz" "target/aarch64-apple-darwin/release/turunmap"
zip "TurunMap-V$TURUNMAP_VERSION-x86_64-pc-windows-gnu.zip" "target/x86_64-pc-windows-gnu/release/turunmap.exe"
zip "TurunMap-V$TURUNMAP_VERSION-x86_64-pc-windows-msvc.zip" "target/x86_64-pc-windows-msvc/release/turunmap.exe"

echo "Done! Version $TURUNMAP_VERSION is ready for upload!"
