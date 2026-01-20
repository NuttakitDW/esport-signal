.PHONY: db run build clean

# Open database in DB Browser for SQLite
db:
	open -a "DB Browser for SQLite" data/signals.db

# Run the application
run:
	cargo run

# Build the application
build:
	cargo build --release

# Clean build artifacts
clean:
	cargo clean
