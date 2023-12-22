cd "$(dirname "$0")"
mkdir -p build
clang++ -fopenmp -std=c++11 -O3 main.cpp cases/prime.cpp cases/quicksort.cpp cases/sum.cpp -o build/main
