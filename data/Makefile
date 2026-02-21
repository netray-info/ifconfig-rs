all:

regexes.yaml:
	curl -s https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml -o $@

tor_exit_nodes.txt:
	curl -s https://check.torproject.org/torbulkexitlist -o $@

feodo_botnet_ips.txt:
	curl -s https://feodotracker.abuse.ch/downloads/ipblocklist.txt -o $@

vpn_ranges.txt:
	curl -s https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/vpn/ipv4.txt > $@
	curl -s https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/vpn/ipv6.txt >> $@

cloud_provider_ranges.jsonl: .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt
	jq -c '.prefixes[] | {cidr: .ip_prefix, provider: "aws", service: .service, region: .region}' .aws.json > $@
	jq -c '.ipv6_prefixes[] | {cidr: .ipv6_prefix, provider: "aws", service: .service, region: .region}' .aws.json >> $@
	jq -c '.prefixes[] | {cidr: .ipv4Prefix, provider: "gcp", service: .service, region: .scope}' .gcp.json >> $@
	jq -c '.prefixes[] | select(.ipv6Prefix) | {cidr: .ipv6Prefix, provider: "gcp", service: .service, region: .scope}' .gcp.json >> $@
	while IFS= read -r cidr; do printf '{"cidr":"%s","provider":"cloudflare","service":null,"region":null}\n' "$$cidr"; done < .cloudflare-v4.txt >> $@
	while IFS= read -r cidr; do printf '{"cidr":"%s","provider":"cloudflare","service":null,"region":null}\n' "$$cidr"; done < .cloudflare-v6.txt >> $@
	rm -f .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt

.aws.json:
	curl -s https://ip-ranges.amazonaws.com/ip-ranges.json -o $@

.gcp.json:
	curl -s https://www.gstatic.com/ipranges/cloud.json -o $@

.cloudflare-v4.txt:
	curl -s https://www.cloudflare.com/ips-v4 -o $@

.cloudflare-v6.txt:
	curl -s https://www.cloudflare.com/ips-v6 -o $@

DATA_FILES = GeoLite2-ASN.mmdb GeoLite2-City.mmdb regexes.yaml tor_exit_nodes.txt \
	feodo_botnet_ips.txt vpn_ranges.txt cloud_provider_ranges.jsonl

build: $(DATA_FILES)
	docker buildx build --platform linux/amd64,linux/arm64 \
		--tag ghcr.io/lukaspustina/ifconfig-rs-data:latest \
		--load \
		.

push: $(DATA_FILES)
	docker buildx build --platform linux/amd64,linux/arm64 \
		--tag ghcr.io/lukaspustina/ifconfig-rs-data:latest \
		--push \
		.

clean: clean_regexes clean_tor clean_feodo clean_vpn clean_cloud
	rm -f *.mmdb

clean_regexes:
	rm -f regexes.yaml

clean_tor:
	rm -f tor_exit_nodes.txt

clean_feodo:
	rm -f feodo_botnet_ips.txt

clean_vpn:
	rm -f vpn_ranges.txt

clean_cloud:
	rm -f cloud_provider_ranges.jsonl .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt

.PHONY: all build push clean clean_regexes clean_tor clean_feodo clean_vpn clean_cloud
