.PHONY: server build trunk clean

# Run Hugo dev server
server:
	hugo server --bind 0.0.0.0 --port 1313 --baseURL http://localhost:1313

# Build WASM app (requires Rust + Trunk)
trunk:
	cd teshi-app && trunk build --release

# Full production build: WASM + Hugo
build: trunk
	hugo --gc --minify --baseURL https://teshi.org/

# Clean build artifacts
clean:
	rm -rf public/ teshi-app/target/
