test:
    cargo make check

build:
    cargo make build

sync:
    cargo make sync

clean:
    cargo clean

ci: test
