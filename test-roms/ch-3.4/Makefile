# You need to have LLVM installed to run this Makefile.
#
# On macOS, you can install LLVM using Homebrew:
# brew install llvm
#
# And export the path to LLVM in your shell configuration file (e.g., ~/.bash_profile):
# echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.bash_profile

all: part1

part1: part1.o
	llvm-objcopy -O binary -j .text part1.o part1

part1.o: part1.s
	llvm-mc -arch=mipsel  -filetype=obj part1.s -o part1.o
