# Use the official Rust image as a base
FROM rust:latest

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

ENV PATH=$HOME/.local/share/dfx/bin:$PATH

# Install cargo-make
RUN cargo install cargo-make

# Display the versions of cargo and cargo-make
RUN cargo --version && cargo-make --version

# Set the entrypoint to cargo-make by default
ENTRYPOINT ["cargo-make"]
