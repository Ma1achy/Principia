"""myllari_formula.py: Mylläri et al. 2018 eqs (11)-(13), arithmetic only. Run: python3 -I myllari_formula.py"""
def f(e, c):
    return 1 - 2 / 3 * e * (1 - .5 * e**2) - .3 * c * (1 - .5 * e + 2 * c * (1 - 2.5 * e**1.5 - c))
def g(m1, m2, m3):
    return 1 + m3 / (m1 + m2)
def Q(A, e, c, m1, m2, m3, eo, N=1e4, lam=1.0):
    return A * (lam * N**.5 / (1 - eo)) ** (1 / 6) * (f(e, c) * g(m1, m2, m3)) ** (1 / 3)
print("The abstract and p. 836: 'At the limit of ein = i = m3 = 0, f g = 1.' Eq. (11) as printed:")
print(f"  f(0, cos i = 1) = {f(0, 1):.6f}; f(0, cos i = 0) = {f(0, 0):.6f}; f(0, cos i = -1) = {f(0, -1):.6f}; g(m3 = 0) = 1")
print(f"  (lambda sqrt(N))^(1/6) at N = 1e4: {(1e4**.5)**(1/6):.6f}; 10^(1/3) = {10**(1/3):.6f}")
print("Fig. 5 (ein = 0.6; m = 1.5, 0.5, 0.5; eout = 0.5; A = 1.15, 1.45, 1.75). Figure's axis read by eye at cos i = +1: about 3.2, 4.0, 4.75")
for c in (-1, -.5, 0, .5, 1):
    print(f"  cos i = {c:+.1f}: " + ", ".join(f"{Q(A, .6, c, 1.5, .5, .5, .5):.3f}" for A in (1.15, 1.45, 1.75)))
print("Figs 7 and 8 (ein = 0; m = 1, 1, 0.1), caption 7 'ein = eout = 0', caption 8 'eout = 0.3'. Their drawn curves look the same; by eye at cos i = +1: about 2.35, 3.0, 3.6")
for eo in (0, .3):
    print(f"  eout = {eo}: cos i = +1: " + ", ".join(f"{Q(A, 0, 1, 1, 1, .1, eo):.3f}" for A in (1.15, 1.45, 1.75)))
print("Prograde and retrograde at ein = 0 (the shape V-11 asks about; A, masses, eout as stated):")
print(f"  (f(0,+1)/f(0,-1))^(1/3) = {(f(0,1)/f(0,-1))**(1/3):.3f}; the book's 4.8/2.6 = {4.8/2.6:.3f} (the book gives no masses or eout: not a comparison)")
for A in (1.45, 1.6, 2.4):
    print(f"  A = {A}, m = 1.5, 0.5, 0.5, eout = 0.5: Qst(+1) = {Q(A,0,1,1.5,.5,.5,.5):.3f}, Qst(-1) = {Q(A,0,-1,1.5,.5,.5,.5):.3f}")
