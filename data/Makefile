all:

get_all: geoip_mmdbs regexes.yaml tor_exit_nodes.txt feodo_botnet_ips.txt vpn_ranges.txt cloud_provider_ranges.jsonl \
	datacenter_ranges.txt bot_ranges.jsonl spamhaus_drop.txt

geoip_mmdbs: .geoip.conf
	geoipupdate -f .geoip.conf -d .

regexes.yaml:
	curl -s https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml -o $@

tor_exit_nodes.txt:
	curl -s https://check.torproject.org/torbulkexitlist -o $@

feodo_botnet_ips.txt:
	curl -s https://feodotracker.abuse.ch/downloads/ipblocklist.txt -o $@

vpn_ranges.txt:
	curl -s https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/vpn/ipv4.txt > $@
	curl -s https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/vpn/ipv6.txt >> $@

cloud_provider_ranges.jsonl: .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt .oracle.json .fastly.json .digitalocean.csv .linode.csv .github.json .google-services.json .azure.txt
	jq -c '.prefixes[] | {cidr: .ip_prefix, provider: "aws", service: .service, region: .region}' .aws.json > $@
	jq -c '.ipv6_prefixes[] | {cidr: .ipv6_prefix, provider: "aws", service: .service, region: .region}' .aws.json >> $@
	jq -c '.prefixes[] | {cidr: .ipv4Prefix, provider: "gcp", service: .service, region: .scope}' .gcp.json >> $@
	jq -c '.prefixes[] | select(.ipv6Prefix) | {cidr: .ipv6Prefix, provider: "gcp", service: .service, region: .scope}' .gcp.json >> $@
	while IFS= read -r cidr; do printf '{"cidr":"%s","provider":"cloudflare","service":null,"region":null}\n' "$$cidr"; done < .cloudflare-v4.txt >> $@
	while IFS= read -r cidr; do printf '{"cidr":"%s","provider":"cloudflare","service":null,"region":null}\n' "$$cidr"; done < .cloudflare-v6.txt >> $@
	jq -c '.regions[] | .region as $$r | .cidrs[] | .cidr as $$c | {cidr: $$c, provider: "oracle", service: null, region: $$r}' .oracle.json >> $@
	jq -c '.addresses[] | {cidr: ., provider: "fastly", service: null, region: null}' .fastly.json >> $@
	jq -c '.ipv6_addresses[] | {cidr: ., provider: "fastly", service: null, region: null}' .fastly.json >> $@
	awk -F, 'NR>1 && $$1 ~ /\// {gsub(/^ +| +$$/, "", $$1); gsub(/^ +| +$$/, "", $$3); printf "{\"cidr\":\"%s\",\"provider\":\"digitalocean\",\"service\":null,\"region\":\"%s\"}\n", $$1, $$3}' .digitalocean.csv >> $@
	awk -F, 'NR>1 && $$1 ~ /\// {gsub(/^ +| +$$/, "", $$1); gsub(/^ +| +$$/, "", $$3); printf "{\"cidr\":\"%s\",\"provider\":\"linode\",\"service\":null,\"region\":\"%s\"}\n", $$1, $$3}' .linode.csv >> $@
	jq -c 'to_entries[] | select(.value | type == "array") | .key as $$svc | .value[] | select(type == "string" and contains("/")) | {cidr: ., provider: "github", service: $$svc, region: null}' .github.json >> $@
	jq -c '.prefixes[] | if .ipv4Prefix then {cidr: .ipv4Prefix, provider: "google-services", service: null, region: null} elif .ipv6Prefix then {cidr: .ipv6Prefix, provider: "google-services", service: null, region: null} else empty end' .google-services.json >> $@
	while IFS= read -r cidr; do printf '{"cidr":"%s","provider":"azure","service":null,"region":null}\n' "$$cidr"; done < .azure.txt >> $@
	rm -f .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt .oracle.json .fastly.json .digitalocean.csv .linode.csv .github.json .google-services.json .azure.txt

.aws.json:
	curl -s https://ip-ranges.amazonaws.com/ip-ranges.json -o $@

.gcp.json:
	curl -s https://www.gstatic.com/ipranges/cloud.json -o $@

.cloudflare-v4.txt:
	curl -s https://www.cloudflare.com/ips-v4 -o $@

.cloudflare-v6.txt:
	curl -s https://www.cloudflare.com/ips-v6 -o $@

.oracle.json:
	curl -s https://docs.oracle.com/en-us/iaas/tools/public_ip_ranges.json -o $@

.fastly.json:
	curl -s https://api.fastly.com/public-ip-list -o $@

.digitalocean.csv:
	curl -s https://www.digitalocean.com/geo/google.csv -o $@

.linode.csv:
	curl -s https://geoip.linode.com/ -o $@

.github.json:
	curl -s https://api.github.com/meta -o $@

.google-services.json:
	curl -s https://www.gstatic.com/ipranges/goog.json -o $@

.azure.txt:
	curl -s https://cloud-ip-ranges.com/download/azure.txt -o $@

datacenter_ranges.txt:
	curl -s https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/datacenter/ipv4.txt -o $@

bot_ranges.jsonl: .googlebot.json .bingbot.json .applebot.json .gptbot.txt
	jq -c '.prefixes[] | if .ipv4Prefix then {cidr: .ipv4Prefix, provider: "googlebot"} elif .ipv6Prefix then {cidr: .ipv6Prefix, provider: "googlebot"} else empty end' .googlebot.json > $@
	jq -c '.prefixes[] | if .ipv4Prefix then {cidr: .ipv4Prefix, provider: "bingbot"} elif .ipv6Prefix then {cidr: .ipv6Prefix, provider: "bingbot"} else empty end' .bingbot.json >> $@
	jq -c '.prefixes[] | if .ipv4Prefix then {cidr: .ipv4Prefix, provider: "applebot"} elif .ipv6Prefix then {cidr: .ipv6Prefix, provider: "applebot"} else empty end' .applebot.json >> $@
	while IFS= read -r cidr; do case "$$cidr" in \#*|"") continue;; esac; printf '{"cidr":"%s","provider":"gptbot"}\n' "$$cidr"; done < .gptbot.txt >> $@
	rm -f .googlebot.json .bingbot.json .applebot.json .gptbot.txt

.googlebot.json:
	curl -s https://developers.google.com/search/apis/ipranges/googlebot.json -o $@

.bingbot.json:
	curl -s https://www.bing.com/toolbox/bingbot.json -o $@

.applebot.json:
	curl -s https://search.developer.apple.com/applebot.json -o $@

.gptbot.txt:
	curl -s https://openai.com/gptbot-ranges.txt -o $@

spamhaus_drop.txt:
	{ curl -s https://www.spamhaus.org/drop/drop.txt; \
	  curl -s https://www.spamhaus.org/drop/edrop.txt; \
	  curl -s https://www.spamhaus.org/drop/dropv6.txt; } \
	| sed -e 's/ ;.*//' -e '/^;/d' -e '/^$$/d' > $@

DATA_FILES = GeoLite2-ASN.mmdb GeoLite2-City.mmdb regexes.yaml tor_exit_nodes.txt \
	feodo_botnet_ips.txt vpn_ranges.txt cloud_provider_ranges.jsonl \
	datacenter_ranges.txt bot_ranges.jsonl spamhaus_drop.txt

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

clean: clean_regexes clean_tor clean_feodo clean_vpn clean_cloud clean_datacenter clean_bot clean_spamhaus
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
	rm -f cloud_provider_ranges.jsonl .aws.json .gcp.json .cloudflare-v4.txt .cloudflare-v6.txt \
		.oracle.json .fastly.json .digitalocean.csv .linode.csv .github.json .google-services.json .azure.txt

clean_datacenter:
	rm -f datacenter_ranges.txt

clean_bot:
	rm -f bot_ranges.jsonl .googlebot.json .bingbot.json .applebot.json .gptbot.txt

clean_spamhaus:
	rm -f spamhaus_drop.txt

.PHONY: all get_all build push clean clean_regexes clean_tor clean_feodo clean_vpn clean_cloud \
	clean_datacenter clean_bot clean_spamhaus
