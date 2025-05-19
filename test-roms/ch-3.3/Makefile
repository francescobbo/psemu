# You need to have LLVM installed to run this Makefile.
#
# On macOS, you can install LLVM using Homebrew:
# brew install llvm
#
# And export the path to LLVM in your shell configuration file (e.g., ~/.bash_profile):
# echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.bash_profile

all: rom

rom: rom.o
	llvm-objcopy -O binary -j .text rom.o rom

rom.o: stress-test.s
	llvm-mc -arch=mipsel  -filetype=obj stress-test.s -o rom.o
