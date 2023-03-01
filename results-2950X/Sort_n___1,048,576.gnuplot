set title "Sort (n = 1,048,576)"
set terminal pdf size 2.6,2.6
set output "./results/Sort_n___1,048,576.pdf"
set key on
set key top left Left reverse
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Threads"
set yrange [0:17]
set ylabel "Speedup"
plot './results/Sort_n___1,048,576.dat' using 1:2 title "Sequential partition" ls 9 lw 1 pointsize 0.6 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:3 title "Work stealing" ls 5 lw 1 pointsize 0.6 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:4 title "Our" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___1,048,576.dat' using 1:5 title "Our (specialized loop)" ls 4 lw 1 pointsize 0.7 with linespoints
