
# get designs from ./hw/*.sv
DESIGN_FILES=$(shell find ./hw -name "*.fir")

# assert that there is only one design file, that is len(DESIGN_FILES) == 1
$(if $(shell test $(shell echo $(DESIGN_FILES) | wc -w) -ne 1), $(error "Expected exactly one design file in ./hw/*.fir"))

# print all design files
$(info FIR file: $(DESIGN_FILES))

# Extract the design name without suffix from the FIR file
DESIGN_NAME=$(basename $(notdir $(DESIGN_FILES)))

DESIGN_FIR=./hw/$(DESIGN_NAME).fir
DESIGN_FIR_BACKPORTED=./$(DESIGN_NAME).backported.fir

DESIGN_MLIR=./hw/$(DESIGN_NAME).mlir

DESIGN_LL=./hw/$(DESIGN_NAME).ll

DESIGN_FAKE_DRIVER=./hw/$(DESIGN_NAME).fake_driver.cpp
DESIGN_HEADER=./$(DESIGN_NAME).h

DESIGN_DRIVER=./tb.cpp
DESIGN_O=./$(DESIGN_NAME).o
DESIGN_BIN=./$(DESIGN_NAME)

$(info Design name: $(DESIGN_NAME))


FIR_BACKPORT_SCRIPT=./fir-backport.py
ifeq ($(shell which firtool-ksim),)
$(error "firtool-ksim not found")
endif
FIRTOOL=$(shell which firtool-ksim)
ifeq ($(shell which ksim),)
$(error "ksim not found")
endif
KSIM=$(shell which ksim)
ifeq ($(shell which llc-ksim),)
$(error "llc-ksim not found")
endif
LLC=$(shell which llc-ksim)
ifeq ($(shell which clang++),)
$(error "clang++ not found")
endif
CLANGPP=$(shell which clang++)

$(info firtool: $(FIRTOOL))
$(info ksim: $(KSIM))
$(info fir-backport.py: $(FIR_BACKPORT_SCRIPT))
$(info llc: $(LLC))
$(info clang++: $(CLANGPP))

all: $(DESIGN_BIN) $(DESIGN_FIR_BACKPORTED) $(DESIGN_MLIR) $(DESIGN_LL) $(DESIGN_O)

$(DESIGN_FIR_BACKPORTED): $(DESIGN_FIR)
	python3 $(FIR_BACKPORT_SCRIPT) $(DESIGN_FIR) -o $(DESIGN_FIR_BACKPORTED)

$(DESIGN_MLIR): $(DESIGN_FIR_BACKPORTED)
	$(FIRTOOL) --ir-hw --disable-all-randomization $(DESIGN_FIR_BACKPORTED) -o $(DESIGN_MLIR)

$(DESIGN_LL): $(DESIGN_MLIR)
	$(KSIM) $(DESIGN_MLIR) -v -o $(DESIGN_LL) --out-header=$(DESIGN_HEADER) --out-driver=$(DESIGN_FAKE_DRIVER)

$(DESIGN_O): $(DESIGN_LL)
	$(LLC) --relocation-model=dynamic-no-pic -O2 -filetype=obj $(DESIGN_LL) -o $(DESIGN_O)

$(DESIGN_BIN): $(DESIGN_O) $(DESIGN_DRIVER)
	$(CLANGPP) -O2 $(DESIGN_O) $(DESIGN_DRIVER) -o $(DESIGN_BIN)
