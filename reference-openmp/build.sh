cd "$(dirname "$0")"
mkdir -p build
clang++ -fopenmp -std=c++11 -O3 -march=native main.cpp cases/compact.cpp cases/prime.cpp cases/quicksort.cpp cases/scan.cpp cases/sum.cpp -o build/main
