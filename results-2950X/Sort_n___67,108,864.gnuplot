set title "Sort (n = 67,108,864)"
set terminal pdf size 2.3,2.43
set output "./results/Sort_n___67,108,864.pdf"
set key off
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set yrange [0:18]
set format y ""
plot './results/Sort_n___67,108,864.dat' using 1:2 title "Sequential partition" ls 9 lw 1 pointsize 0.6 with linespoints, \
  './results/Sort_n___67,108,864.dat' using 1:3 title "Work stealing" ls 5 lw 1 pointsize 0.6 with linespoints, \
  './results/Sort_n___67,108,864.dat' using 1:4 title "Our" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/Sort_n___67,108,864.dat' using 1:5 title "Our (specialized loop)" ls 4 lw 1 pointsize 0.7 with linespoints
