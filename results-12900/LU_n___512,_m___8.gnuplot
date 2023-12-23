set title "LU (n = 512, m = 8)"
set terminal pdf size 3.2,2.8
set output "./results/LU_n___512,_m___8.pdf"
set key on
set key top left Left reverse
set xrange [1:24]
set xtics (1, 4, 8, 12, 16, 20, 24)
set xlabel "Number of threads"
set yrange [0:14]
set ylabel "Speedup"
plot './results/LU_n___512,_m___8.dat' using 1:2 title "Work stealing" pointsize 0.7 lw 1 pt 6 linecolor rgb "#5B2182" with linespoints, \
  './results/LU_n___512,_m___8.dat' using 1:3 title "OpenMP" pointsize 0.7 lw 1 pt 4 linecolor rgb "#001240" with linespoints, \
  './results/LU_n___512,_m___8.dat' using 1:4 title "Work assisting (our)" pointsize 0.4 lw 2 pt 7 linecolor rgb "#C00A35" with linespoints
