# Use the official Rust image as a base
FROM rust:latest

ENV HOME=/code
RUN mkdir $HOME
WORKDIR $HOME

# Install dfx
RUN XDG_DATA_HOME=/usr/local DFXVM_INIT_YES=yes sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"

ENV PATH=/usr/local/dfx/bin:$PATH

# Install cargo-make
RUN cargo install cargo-make

# Display the version of cargo and cargo make
RUN cargo --version && cargo make --version
