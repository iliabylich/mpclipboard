set dotenv-load

mod generic-client 'generic-client'
mod server         'server'
mod poll-cli       'poll-cli'
mod macos          'macos'
mod linux          'linux'
mod android        'android'

clippy:
    cd shared && cargo clippy
    cd generic-client && cargo clippy
    cd server && cargo clippy
