all: src/codec/generated.rs

src/codec/generated.rs: target/debug/jntajis-codegen target/data/jissyukutaimap1_0_0.xlsx target/data/mji.00602.xlsx target/data/MJShrinkMap.1.2.0.json
	OUT_DIR=src/codec target/debug/jntajis-codegen

target/debug/jntajis-codegen: src/codegen/main.rs
	cargo build -F codegen --bin jntajis-codegen

target/data/mji.00602.xlsx:
	install -d target/data
	curl -L -o $@ https://moji.or.jp/wp-content/uploads/2024/01/mji.00602.xlsx

target/data/MJShrinkMap.1.2.0.json: target/data/MJShrinkMapVer.1.2.0.zip
	install -d target/data
	unzip -qq -c $< $(notdir $@) > "$@"

target/data/MJShrinkMapVer.1.2.0.zip:
	install -d target/data
	curl -L -o $@ https://moji.or.jp/wp-content/mojikiban/oscdl/MJShrinkMapVer.1.2.0.zip

target/data/jissyukutaimap1_0_0.xlsx: target/data/syukutaimap1_0_0.zip
	install -d target/data
	unzip -qq -c $< $(notdir $@) > "$@"

target/data/syukutaimap1_0_0.zip:
	install -d target/data
	curl -A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.101 Safari/537.36." -L -o $@ https://www.houjin-bangou.nta.go.jp/download/images/syukutaimap1_0_0.zip

.PHONY: all
