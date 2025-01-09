ROOT_DIR=$(shell dirname $(shell dirname $(shell pwd)))

TEST_DESIGN_DIR = tests/
# find all .cmt files that not start with . in the tests directory, remove the directory name and .cmt extension
TEST_DESIGN_NAMES = $(shell find $(TEST_DESIGN_DIR) -name "*.cmt" | sed 's/.*\///;s/\.cmt//' | grep -v '^\.')
# print the test design names
$(info $(TEST_DESIGN_NAMES))


TEST_DIR=$(shell pwd)
# Generate test design files with .cmt extension
TEST_DESIGN_FILES = $(addprefix $(TEST_DESIGN_DIR),$(TEST_DESIGN_NAMES))

# print the test design files
# $(info $(TEST_DESIGN_FILES))
$(info $(addsuffix .cmtir, $(basename $(TEST_DESIGN_FILES))))

$(info $(ROOT_DIR))

# for each TEST_DESIGN_DIR/x.cmt, generate a target to run the cmt2 compiler on it and output x.cmtir
$(foreach design, $(TEST_DESIGN_FILES), \
	$(eval $(design).cmtir: $(design).cmt ; cd $(ROOT_DIR) && CMTLIB=$(ROOT_DIR) cargo run --bin cmt2 -- $(TEST_DIR)/$(design).cmt -o $(TEST_DIR)/$(design).cmtir))

cmtirs: $(addsuffix .cmtir, $(basename $(TEST_DESIGN_FILES)))

clean:
	rm -f $(addsuffix .cmtir, $(basename $(TEST_DESIGN_FILES)))