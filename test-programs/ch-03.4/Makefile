# See the Makefile in ch-03.3 for how to install LLVM.

all: binary

binary: binary.o
	llvm-objcopy -O binary -j .text binary.o binary

binary.o: main.s
	llvm-mc -arch=mipsel  -filetype=obj main.s -o binary.o
