set title "m = 8"
set terminal pdf size 2.2,2.0
set output "./results/LUD_n___512,_m___8.pdf"
set key off
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Number of threads"
set yrange [0:9]
set ytics (0, 2, 4, 6, 8)
set ylabel "Speedup"
plot './results/LUD_n___512,_m___8.dat' using 1:2 title "Work stealing" pointsize 0.7 lw 1 pt 6 linecolor rgb "#5B2182" with linespoints, \
  './results/LUD_n___512,_m___8.dat' using 1:3 title "OpenMP (loops)" pointsize 0.7 lw 1 pt 4 linecolor rgb "#001240" with linespoints, \
  './results/LUD_n___512,_m___8.dat' using 1:4 title "OpenMP (tasks)" pointsize 0.7 lw 1 pt 12 linecolor rgb "#F3965E" with linespoints, \
  './results/LUD_n___512,_m___8.dat' using 1:5 title "Work assisting (our)" pointsize 0.4 lw 2 pt 7 linecolor rgb "#C00A35" with linespoints
