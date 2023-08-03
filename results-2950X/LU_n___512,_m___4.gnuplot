set title "LU (n = 512, m = 4)"
set terminal pdf size 3.2,2.8
set output "./results/LU_n___512,_m___4.pdf"
set key on
set key top left Left reverse
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Number of threads"
set yrange [0:18]
set ylabel "Speedup"
plot './results/LU_n___512,_m___4.dat' using 1:2 title "Work stealing" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/LU_n___512,_m___4.dat' using 1:3 title "OpenMP" ls 5 lw 1 pointsize 0.7 with linespoints, \
  './results/LU_n___512,_m___4.dat' using 1:4 title "Work assisting (our)" ls 7 lw 2 pointsize 0.4 with linespoints
