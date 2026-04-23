mod generic-client 'generic-client/Justfile'
mod server         'server/Justfile'
mod poll-cli       'poll-cli/Justfile'
mod macos          'macos/Justfile'

clippy:
    cd generic-client && cargo clippy
    cd server && cargo clippy
