# You need to have LLVM installed to run this Makefile.

# On Linux, you can install LLVM using your package manager, for example: sudo
# apt-get install llvm

# On macOS, you can install LLVM using Homebrew:
# brew install llvm
#
# And export the path to LLVM in your shell configuration file (e.g., ~/.bash_profile):
# echo 'export PATH="/opt/homebrew/opt/llvm/bin:$PATH"' >> ~/.bash_profile

# On Windows, I think your best bet is to use WSL (Windows Subsystem for Linux)
# and follow the Linux instructions.

all: binary

binary: binary.o
	llvm-objcopy -O binary -j .text binary.o binary

binary.o: main.s
	llvm-mc -arch=mipsel  -filetype=obj main.s -o binary.o
