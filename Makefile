all: check unit integration

doctoc:
	doctoc --maxlevel 2 Readme.md

check:
	cargo $@

build: frontend
	cargo $@

clean:
	cargo $@

fmt:
	cargo $@

clippy:
	cargo $@

tests: unit integration acceptance

unit: frontend
	cargo test --lib --no-fail-fast
	cargo test

frontend:
	cd frontend && npm ci && npm run build

frontend-dev:
	cd frontend && npm run dev

dev:
	cargo run -- ifconfig.toml

integration:
	$(MAKE) -C tests $@

acceptance:
	$(MAKE) -C tests $@

docker-build:
	docker build . --tag ifconfig-rs:$$(cargo read-manifest | jq ".version" -r)

push_to_prod:
	git checkout prod
	git merge master
	git push
	git checkout master

.PHONY: tests frontend frontend-dev dev
