#!/bin/bash
# This is optional and highly opinionated
sudo dnf -y install util-linux-user
sudo chsh -s $(which zsh)
sudo dnf -y install tmux # because life without tmux is painful

wget https://github.com/robbyrussell/oh-my-zsh/raw/master/tools/install.sh -O - | zsh || true
git clone https://github.com/zsh-users/zsh-autosuggestions ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-autosuggestions
git clone https://github.com/zsh-users/zsh-syntax-highlighting.git ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-syntax-highlighting

# finally set up the new shell
zsh -c "cp /workspaces/edgeless_mvp/scripts/.zshrc $HOME; source $HOME/.zshrc"

# Persist the zsh_history over container rebuilds
# export SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.zsh_history" \
#     && mkdir /commandhistory \
#     && touch /commandhistory/.zsh_history \
#     && chown -R $USERNAME /commandhistory \
#     && echo "$SNIPPET" >> "/home/edgeless/.zshrc"