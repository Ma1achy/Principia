// pilot.cpp: our driver for the RQ-598 Brutus cost pilot ("Brutus, run here"; not the papers' code or runs).
//
// Links the UNMODIFIED Star/Cluster/Bulirsch_Stoer/Brutus objects of pgbreen/NvMgeneration @ 98906f4.
// It exists because that commit's main.cpp (a) reads the word length L_w and never applies it (the build runs at
// MPFR's default 53 bits whatever L_w is), (b) offers no escape test, no step count and no wall cap.
// Here mpfr's default precision is set to L_w before any mpreal is made, and the step loop below is a line-for-line
// copy of Brutus::evolve (Brutus.cpp), with a per-step hook.
//
//   pilot gg1 <i> <j> <eps> <Lw> <tmax> <wall_cap_s> [pmax]
//   pilot br1 <x2x> <x2y> <eps> <Lw> <tend> <traj_out|-> [pmax] [wall_cap_s]
//   pilot --header
#include <iostream>
#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <ctime>
#include <string>
#include <sys/resource.h>
using namespace std;

#include "mpreal.h"
using namespace mpfr;

#include "Brutus.h"

static double wall_now() {
  timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return ts.tv_sec + 1e-9 * ts.tv_nsec;
}
static double cpu_now() {
  rusage ru;
  getrusage(RUSAGE_SELF, &ru);
  return ru.ru_utime.tv_sec + 1e-6 * ru.ru_utime.tv_usec;
}

struct Snap {
  double m[3], r[3][2], v[3][2];
};
static void snap(const Cluster &cl, Snap &s) {
  for (int k = 0; k < 3; ++k) {
    s.m[k] = cl.s[k].m.toDouble();
    for (int c = 0; c < 2; ++c) {
      s.r[k][c] = cl.s[k].r[c].toDouble();
      s.v[k][c] = cl.s[k].v[c].toDouble();
    }
  }
}

// Boekholt et al. 2020, Appendix A: the single is permanently escaped if (1) it is more than 10 distance units from
// the binary's centre of mass, (2) it moves away from the binary, (3) its energy is positive. We also require the
// other two to be a bound pair. G = 1. On success: the single's index and its position/velocity relative to the
// binary centre of mass, and the total mass of the two-body (single + binary) problem.
static bool escaped(const Snap &s, int &single, double R[2], double V[2], double &mu_g) {
  for (int k = 0; k < 3; ++k) {
    int i = (k + 1) % 3, j = (k + 2) % 3;
    double mb = s.m[i] + s.m[j];
    double dx = s.r[i][0] - s.r[j][0], dy = s.r[i][1] - s.r[j][1];
    double dvx = s.v[i][0] - s.v[j][0], dvy = s.v[i][1] - s.v[j][1];
    double rb = std::sqrt(dx * dx + dy * dy);
    double mub = s.m[i] * s.m[j] / mb;
    double Eb = 0.5 * mub * (dvx * dvx + dvy * dvy) - s.m[i] * s.m[j] / rb;
    if (Eb >= 0) continue;
    double Xb[2], Vb[2];
    for (int c = 0; c < 2; ++c) {
      Xb[c] = (s.m[i] * s.r[i][c] + s.m[j] * s.r[j][c]) / mb;
      Vb[c] = (s.m[i] * s.v[i][c] + s.m[j] * s.v[j][c]) / mb;
      R[c] = s.r[k][c] - Xb[c];
      V[c] = s.v[k][c] - Vb[c];
    }
    double Rn = std::sqrt(R[0] * R[0] + R[1] * R[1]);
    if (Rn <= 10.0) continue;
    if (R[0] * V[0] + R[1] * V[1] <= 0) continue;
    double mus = s.m[k] * mb / (s.m[k] + mb);
    double Es = 0.5 * mus * (V[0] * V[0] + V[1] * V[1]) - s.m[k] * mb / Rn;
    if (Es <= 0) continue;
    single = k;
    mu_g = s.m[k] + mb;
    return true;
  }
  return false;
}

// Time since the single's pericentre passage relative to the binary centre of mass, treating the single and the
// binary as point masses on a Kepler orbit (our reading of Appendix A: "retracing their steps to the moment when
// the binary+single were closest"). Hyperbolic branch; Barker's equation when e is within 1e-9 of 1.
static long double time_since_pericentre(const double R[2], const double V[2], double mu_g) {
  long double r = sqrtl((long double)R[0] * R[0] + (long double)R[1] * R[1]);
  long double v2 = (long double)V[0] * V[0] + (long double)V[1] * V[1];
  long double h = (long double)R[0] * V[1] - (long double)R[1] * V[0];
  long double eps = 0.5L * v2 - mu_g / r;
  long double e = sqrtl(1.0L + 2.0L * eps * h * h / ((long double)mu_g * mu_g));
  if (e - 1.0L < 1e-9L) {
    long double q = h * h / (2.0L * mu_g);
    long double cth = 2.0L * q / r - 1.0L;
    if (cth > 1.0L) cth = 1.0L;
    if (cth < -1.0L) cth = -1.0L;
    long double D = tanl(0.5L * acosl(cth));
    return sqrtl(2.0L * q * q * q / mu_g) * (D + D * D * D / 3.0L);
  }
  long double a = mu_g / (2.0L * eps);
  long double coshF = (1.0L + r / a) / e;
  if (coshF < 1.0L) coshF = 1.0L;
  long double F = acoshl(coshF);
  return sqrtl(a * a * a / mu_g) * (e * sinhl(F) - F);
}

struct Run {
  Cluster cl;
  Bulirsch_Stoer bs;
  mpreal t, eta;
  long steps;
  bool failed;
};

// One step of Brutus::evolve (Brutus.cpp): dt = eta * cl.dt, clipped to land on t_end, then Bulirsch-Stoer.
static bool one_step(Run &r, const mpreal &t_end) {
  r.cl.calcAcceleration_dt();
  mpreal dt = r.eta * r.cl.dt;
  if (r.t + dt > t_end) dt = t_end - r.t;
  bool converged = r.bs.integrate(r.cl, dt);
  if (!converged) return false;
  r.t += dt;
  r.steps++;
  return true;
}

static void setup_run(Run &r, vector<mpreal> &data, mpreal &tol, int &Lw, int &pmax) {
  mpreal t0 = "0";
  Brutus b(t0, data, tol, Lw, pmax);  // used only for Brutus::get_eta, so eta is the pinned code's own
  r.eta = b.get_eta(tol);
  Cluster c(data);
  c.eps2 = "0";
  r.cl = c;
  Bulirsch_Stoer bs(tol);
  r.bs = bs;
  r.bs.set_p_max(pmax);
  r.t = "0";
  r.steps = 0;
  r.failed = false;
}

static mpreal energy(Run &r) { return r.cl.energies()[0]; }

static int run_gg1(int argc, char **argv) {
  int gi = atoi(argv[2]), gj = atoi(argv[3]);
  string eps_s = argv[4];
  int Lw = atoi(argv[5]);
  double tmax = atof(argv[6]), cap = atof(argv[7]);
  int pmax = argc > 8 ? atoi(argv[8]) : 4;
  mpreal::set_default_prec(Lw);
  mpreal tol = eps_s.c_str();
  mpreal x = mpreal(gi) / 64, y = mpreal(gj) / 64, half = "0.5";
  vector<mpreal> d(21, mpreal("0"));
  // Agekyan-Anosova frame: m = 1, G = 1, bodies 1 and 2 at (-1/2, 0) and (1/2, 0), body 3 at (x, y), all at rest.
  d[0] = 1; d[1] = -half;
  d[7] = 1; d[8] = half;
  d[14] = 1; d[15] = x; d[16] = y;
  Run r;
  setup_run(r, d, tol, Lw, pmax);
  vector<mpreal> init = r.cl.get_data();
  mpreal E0 = energy(r);
  double w0 = wall_now(), c0 = cpu_now();
  string status = "censored";
  double tf = 0, Tl = -1, eerr = 0;
  mpreal tmaxm = tmax;
  Snap sn;
  int single = -1;
  double R[2], V[2], mug = 0;
  while (r.t < tmaxm) {
    if (!one_step(r, tmaxm)) { status = "bs_not_converged"; break; }
    snap(r.cl, sn);
    if (escaped(sn, single, R, V, mug)) {
      status = "escaped";
      tf = r.t.toDouble();
      Tl = tf - (double)time_since_pericentre(R, V, mug);
      break;
    }
    if ((r.steps & 15) == 0 && wall_now() - w0 > cap) { status = "wall_cap"; break; }
  }
  double wf = wall_now() - w0, cf = cpu_now() - c0;
  double t_reached = r.t.toDouble();
  long stepsf = r.steps;
  if (status == "escaped") eerr = ((energy(r) / E0) - 1).toDouble();
  // Reversibility test (Appendix A): reverse the velocities at t = T_f, integrate to 2 T_f, compare positions.
  string rev = "n/a";
  double log_delta = 0, wr = 0, cr = 0;
  long stepsr = 0;
  if (status == "escaped") {
    for (int k = 0; k < 3; ++k)
      for (int c = 0; c < 3; ++c) r.cl.s[k].v[c] = -r.cl.s[k].v[c];
    mpreal t2 = r.t * 2;
    long s0 = r.steps;
    double w1 = wall_now(), c1 = cpu_now();
    rev = "reversed";
    while (r.t < t2) {
      if (!one_step(r, t2)) { rev = "bs_not_converged"; break; }
      if ((r.steps & 15) == 0 && wall_now() - w1 > cap) { rev = "wall_cap"; break; }
    }
    wr = wall_now() - w1; cr = cpu_now() - c1; stepsr = r.steps - s0;
    if (rev == "reversed") {
      vector<mpreal> fin = r.cl.get_data();
      mpreal s2 = 0;
      for (int k = 0; k < 3; ++k)
        for (int c = 1; c <= 3; ++c) {
          mpreal dd = fin[k * 7 + c] - init[k * 7 + c];
          s2 += dd * dd;
        }
      log_delta = mpfr::log10(mpfr::sqrt(s2)).toDouble();
      rev = log_delta < -3 ? "pass" : "fail";
    }
  }
  printf("gg1,%d,%d,%.10g,%.10g,%s,%d,%s,%.9g,%.9g,%.6g,%ld,%.3f,%.3f,%s,%.4f,%ld,%.3f,%.3f,%.3e,%d,%.5g\n", gi, gj,
         gi / 64.0, gj / 64.0, eps_s.c_str(), Lw, status.c_str(), t_reached, Tl, tf, stepsf, wf, cf, rev.c_str(),
         log_delta, stepsr, wr, cr, eerr, single, mug);
  return 0;
}

static int run_br1(int argc, char **argv) {
  string x2x_s = argv[2], x2y_s = argv[3], eps_s = argv[4];
  int Lw = atoi(argv[5]);
  double tend = atof(argv[6]);
  string out = argv[7];
  int pmax = argc > 8 ? atoi(argv[8]) : 4;
  double cap = argc > 9 ? atof(argv[9]) : 1e30;
  mpreal::set_default_prec(Lw);
  mpreal tol = eps_s.c_str();
  // As the stock main.cpp does, the initial coordinates are read as doubles and widened to mpreal.
  double x2x = atof(x2x_s.c_str()), x2y = atof(x2y_s.c_str());
  mpreal ax = x2x, ay = x2y;  // exact widening of the double
  vector<mpreal> d(21, mpreal("0"));
  d[0] = 1; d[1] = 1; d[2] = 0;
  d[7] = 1; d[8] = ax; d[9] = ay;
  d[14] = 1; d[15] = -mpreal(1) - ax; d[16] = -ay;  // x3 from the barycentre, equal masses
  Run r;
  setup_run(r, d, tol, Lw, pmax);
  mpreal E0 = energy(r);
  FILE *fo = out == "-" ? NULL : fopen(out.c_str(), "w");
  vector<mpreal> q = r.cl.get_data();
  if (fo) fprintf(fo, "%.17g %.17g %.17g %.17g\n", q[1].toDouble(), q[2].toDouble(), q[8].toDouble(), q[9].toDouble());
  double w0 = wall_now(), c0 = cpu_now();
  int nout = (int)llround(tend * 256);
  string status = "ok";
  double eerr = 0, rmin = 1e300;
  for (int k = 1; k <= nout && status == "ok"; ++k) {
    mpreal tk = mpreal(k) / 256;
    while (r.t < tk) {
      if (!one_step(r, tk)) { status = "bs_not_converged"; break; }
      if ((r.steps & 15) == 0 && wall_now() - w0 > cap) { status = "wall_cap"; break; }
      Snap s;
      snap(r.cl, s);
      for (int a = 0; a < 3; ++a)
        for (int b = a + 1; b < 3; ++b) {
          double dx = s.r[a][0] - s.r[b][0], dy = s.r[a][1] - s.r[b][1];
          double rr = std::sqrt(dx * dx + dy * dy);
          if (rr < rmin) rmin = rr;
        }
    }
    if (status != "ok") break;
    q = r.cl.get_data();
    if (fo) fprintf(fo, "%.17g %.17g %.17g %.17g\n", q[1].toDouble(), q[2].toDouble(), q[8].toDouble(), q[9].toDouble());
  }
  double wf = wall_now() - w0, cf = cpu_now() - c0;
  eerr = ((energy(r) / E0) - 1).toDouble();
  if (fo) fclose(fo);
  printf("br1,%s,%s,%s,%d,%s,%.6g,%ld,%.3f,%.3f,%.3e,%.6g\n", x2x_s.c_str(), x2y_s.c_str(), eps_s.c_str(), Lw,
         status.c_str(), r.t.toDouble(), r.steps, wf, cf, eerr, rmin);
  return 0;
}

int main(int argc, char **argv) {
  if (argc >= 2 && string(argv[1]) == "--header") {
    printf("kind,i,j,x,y,eps,Lw,status,t_reached,T,t_escape,steps_fwd,wall_fwd_s,cpu_fwd_s,reversal,log10_delta,steps_rev,"
           "wall_rev_s,cpu_rev_s,energy_relerr,single,mu_g\n");
    printf("kind,x2x,x2y,eps,Lw,status,t_reached,steps,wall_s,cpu_s,energy_relerr,min_pair_sep\n");
    return 0;
  }
  if (argc >= 8 && string(argv[1]) == "gg1") return run_gg1(argc, argv);
  if (argc >= 8 && string(argv[1]) == "br1") return run_br1(argc, argv);
  fprintf(stderr, "usage: see the head of pilot.cpp\n");
  return 2;
}
