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

# Some more tools to make the experience better - comment them out 
# if you don't like it and rebuild the dev container
# --- Start
RUN sudo dnf -y install tmux # because life without tmux is painful
RUN sudo dnf -y install zsh
RUN sudo dnf -y install wget
RUN sudo dnf -y install powerline powerline-fonts # installing the fonts
ENV HOME /home/$USERNAME

# setup the zsh with some goodies
ENV SHELL /bin/zsh
RUN wget https://github.com/robbyrussell/oh-my-zsh/raw/master/tools/install.sh -O - | zsh || true
RUN git clone https://github.com/romkatv/powerlevel10k.git ~/.oh-my-zsh/custom/themes/powerlevel10k
RUN cd $HOME && curl -fsSLO https://raw.githubusercontent.com/romkatv/dotfiles-public/master/.purepower
ADD .zshrc $HOME
ADD .p10k.zsh $HOME
# --- End

ENTRYPOINT ["zsh"]
USER $USERNAME
WORKDIR /home/$USERNAME
