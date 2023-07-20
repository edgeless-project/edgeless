FROM fedora:latest

ARG USERNAME=edgeless
ARG USER_UID=1000
ARG USER_GID=1000

RUN dnf -y install sudo git iputils dnsutils net-tools telnet iproute curl lldb openssl openssl-devel protobuf protobuf-compiler protobuf-devel clang
RUN groupadd --gid ${USER_GID} ${USERNAME} 
RUN useradd -rm -d /home/$USERNAME -s /bin/bash -G wheel -u $USER_UID -g $USER_GID $USERNAME
RUN echo '%wheel ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

RUN su $USERNAME -l -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
RUN su $USERNAME -l -c "rustup target add wasm32-unknown-unknown"
RUN su $USERNAME -l -c "cargo install wasm-tools"

USER $USERNAME
WORKDIR /home/$USERNAME
