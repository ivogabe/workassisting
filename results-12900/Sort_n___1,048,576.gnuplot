set title "Sort (n = 1,048,576)"
set terminal pdf size 3.2,2.8
set output "./results/Sort_n___1,048,576.pdf"
set key on
set key top left Left reverse
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Number of threads"
set yrange [0:18]
set ylabel "Speedup"
plot './results/Sort_n___1,048,576.dat' using 1:2 title "Sequential partition" ls 9 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:3 title "Work stealing" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:4 title "OpenMP (balance threads)" ls 5 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:5 title "OpenMP (oversubscribe)" ls 4 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:6 title "Work assisting (our)" ls 7 lw 2 pointsize 0.4 with linespoints
