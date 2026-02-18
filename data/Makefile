all:

regexes.yaml:
	curl -s https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml -o $@

tor_exit_nodes.txt:
	curl -s https://check.torproject.org/torbulkexitlist -o $@

build: GeoLite2-ASN.mmdb GeoLite2-City.mmdb regexes.yaml tor_exit_nodes.txt
	docker buildx build --platform linux/amd64,linux/arm64 \
		--tag ghcr.io/lukaspustina/ifconfig-rs-data:latest \
		--load \
		.

push: GeoLite2-ASN.mmdb GeoLite2-City.mmdb regexes.yaml tor_exit_nodes.txt
	docker buildx build --platform linux/amd64,linux/arm64 \
		--tag ghcr.io/lukaspustina/ifconfig-rs-data:latest \
		--push \
		.

clean: clean_regexes clean_tor
	rm *.mmdb

clean_regexes:
	rm regexes.yaml

clean_tor:
	rm tor_exit_nodes.txt