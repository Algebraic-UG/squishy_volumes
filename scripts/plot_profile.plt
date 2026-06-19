if (!exists("ARG1")) {
    print "usage: gnuplot -persist plot_profile.plt profile.csv"
    exit
}

csvfile = ARG1

set datafile separator comma
set datafile columnheaders

set terminal qt persist size 1600,900

set xlabel "Frame"
set ylabel "Time [ms]"
set key outside
set title csvfile

plot for [i=2:*:2] \
    csvfile using 1:(column(i)):(column(i+1)) \
    with filledcurves title columnhead(i)

pause mouse close
