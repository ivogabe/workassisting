set title "Primes (2 .. 4,194,306)"
set terminal pdf size 3.2,2.8
set output "./results/Primes_2_.._4,194,306.pdf"
set key off
set xrange [1:32]
set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)
set xlabel "Number of threads"
set yrange [0:18]
set ylabel "Speedup"
plot './results/Primes_2_.._4,194,306.dat' using 1:2 title "Rayon" ls 1 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:3 title "Static" ls 2 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:4 title "Static (pinned)" ls 3 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:5 title "Work stealing" ls 6 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:6 title "OpenMP (static)" ls 5 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:7 title "OpenMP (dynamic)" ls 4 lw 1 pointsize 0.7 with linespoints, \
  './results/Primes_2_.._4,194,306.dat' using 1:8 title "Work assisting (our)" ls 7 lw 2 pointsize 0.4 with linespoints
