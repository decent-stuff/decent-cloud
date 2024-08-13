# Use the official Rust image as a base
FROM rust:latest

ENV HOME=/code \
    XDG_DATA_HOME=/usr/local
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

ENV PATH=/usr/local/dfx/bin:$PATH

# Install deps
RUN apt update && apt install -y libunwind-dev

# Install cargo-make
RUN cargo install cargo-make

# Display the version of cargo and cargo make
RUN cargo --version && cargo make --version
