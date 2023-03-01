set title "Sum array (n = 1,073,743,058)"
set terminal pdf size 2.6,2.6
set output "./results/Sum_array_n___1,073,743,058.pdf"
set key on
set key top left Left reverse
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Threads"
set yrange [0:18]
set ylabel "Speedup"
plot './results/Sum_array_n___1,073,743,058.dat' using 1:2 title "Rayon" ls 1 lw 1 pointsize 0.6 with linespoints, \
  './results/Sum_array_n___1,073,743,058.dat' using 1:3 title "Naive" ls 2 lw 1 pointsize 0.6 with linespoints, \
  './results/Sum_array_n___1,073,743,058.dat' using 1:4 title "Naive (pinned)" ls 3 lw 1 pointsize 0.6 with linespoints, \
  './results/Sum_array_n___1,073,743,058.dat' using 1:5 title "Work stealing" ls 5 lw 1 pointsize 0.6 with linespoints, \
  './results/Sum_array_n___1,073,743,058.dat' using 1:6 title "Our" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/Sum_array_n___1,073,743,058.dat' using 1:7 title "Our (specialized loop)" ls 4 lw 1 pointsize 0.7 with linespoints
