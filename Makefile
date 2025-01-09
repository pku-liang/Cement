
clean:
	rm -f ./*.sv ./*.fir ./*.dot ./*.log ./crates/cmtc/*.sv ./crates/cmtc/*.fir ./crates/cmtc/*.cmtir ./crates/cmtc/*.sv ./crates/cmtir/*.dot
	rm -rf ./tb
	cargo clean