set title "Sort (n = 262,144)"
set terminal pdf size 3.2,2.9
set output "./results/Sort_n___262,144.pdf"
set key on
set key top left Left reverse
set key samplen 2.5
set xrange [1:24]
set xtics (1, 4, 8, 12, 16, 20, 24)
set xlabel "Number of threads"
set yrange [0:16]
set ylabel "Speedup"
plot './results/Sort_n___262,144.dat' using 1:2 title "Sequential partition" pointsize 0.7 lw 1 pt 1 linecolor rgb "#24A793" with linespoints, \
  './results/Sort_n___262,144.dat' using 1:3 title "Work stealing" pointsize 0.7 lw 1 pt 6 linecolor rgb "#5B2182" with linespoints, \
  './results/Sort_n___262,144.dat' using 1:4 title "OpenMP (nested loops)" pointsize 0.7 lw 1 pt 4 linecolor rgb "#001240" with linespoints, \
  './results/Sort_n___262,144.dat' using 1:5 title "OpenMP (tasks)" pointsize 0.7 lw 1 pt 12 linecolor rgb "#F3965E" with linespoints, \
  './results/Sort_n___262,144.dat' using 1:6 title "Work assisting (our)" pointsize 0.4 lw 2 pt 7 linecolor rgb "#C00A35" with linespoints
