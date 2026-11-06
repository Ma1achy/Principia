# Questions for the authors, for the owner's pending emails

*4 Nov 2026 (R-701). The owner sends the emails; agents don't, and this file holds no address (R-668: an author is emailed only as a last resort; these are that). Collected from RQ-612, RQ-613, RQ-615 and RQ-616, now in `docs/archive/review_queue/M0.md`. Each answer, when it comes, is a new ruling's input; nothing here is tuned to it.*

## Boekholt (Boekholt, Portegies Zwart & Valtonen 2020, GG-1)

Beside the per-point lifetimes the email already asks for:

1. **The retrace rule.** Appendix A measures the lifetime "by considering the final snapshot of the forward integration … and then retracing their steps to the moment when the binary+single were closest to the moment of the final ejection". How is that moment picked, and is the escape radius of "10 distance units" in Heggie–Mathieu units? (RQ-612)
2. **The grid convention.** How does a cell of Fig. 1 map to its initial condition: is the plotted value at the cell's centre or a corner, and what are the grid's offset and step? Our (3, 43) is 2.45 dex from the figure's isolated maximum, so a one-cell offset is one candidate. (RQ-612)
3. **The time unit.** Are Fig. 1's times in Heggie–Mathieu units (G = 1, M = 1, E = −1/4)? We read them so, as the paper cites; read as G = M = 1 with no energy scaling, all three of our points move further from the figure. (RQ-612)
4. **`p_max`.** The paper gives none. Which value did the runs use? (RQ-613)
5. **The word length.** Did the code behind the paper apply `L_w` = 1024? The `main.cpp` of `pgbreen/NvMgeneration` @ `98906f4`, as recorded, reads the word length and never applies it (unverified until the source is read again). (RQ-613)

## Breen (Breen, Foley, Boekholt & Portegies Zwart 2020, BR-1)

Beside the full ~10⁴ set the email already asks for:

1. **`p_max` and the word length,** as items 4 and 5 above, for the runs behind the paper. (RQ-613)
2. **The convergence criterion.** Appendix A's eq. (A1) gives δ² as a mean square, and the next sentence calls δ "the mean squared error". Does "average phase distance < 0.1" apply to the time-mean of δ, to the time-mean of δ², or to √ of the time-mean of δ²? (RQ-616)
3. **A licence.** `github.com/pgbreen/NVM` @ `a35c378` has 180 trajectory files and no licence file. May we use them, and may anything derived from them be published? We keep them local and commit nothing from them. (RQ-615)
