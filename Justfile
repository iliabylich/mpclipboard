mod clip           'clip/Justfile'
mod generic-client 'generic-client/Justfile'
mod server         'server/Justfile'

ci:
    @just clip::test
    @just generic-client::check
    @just server::check
