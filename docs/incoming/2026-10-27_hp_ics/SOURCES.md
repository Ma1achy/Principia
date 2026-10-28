# Sources of the committed high-precision initial conditions (`data/`)

Committed by R-631 (27 Oct 2026), which amends R-630 item 6 and closes RQ-579 differently: "commit the
high-precision initial-condition data to the repo, with full citations." Every file under `data/` is the file
`fetch.sh` downloads, byte for byte, at the path `sources.tsv` gives it; each was fetched on 27 Oct 2026 and
matched its recorded sha256 (`bash fetch.sh <empty dir>`: "all 599 files fetched and match sources.tsv"). Nothing
here is ours: the values are the authors' published numerical results, credited to them below. Nothing is
compressed by us; `choreo3.tar.gz` is committed as Simó's server serves it, not extracted.

Where a paper states a licence it is given, as Crossref's record of the DOI (or the proceedings' own page) lists it,
checked 27 Oct 2026. No database page, and no data file, states terms of its own (R-631's reasons; `README.md`,
"Terms").

## hristov2022

- **Authors:** Ivan Hristov, Radoslava Hristova, Igor Puzynin, Taisya Puzynina, Zarif Sharipov, Zafar Tukhliev
- **Title:** *New families of periodic orbits for the planar three-body problem computed with high precision*
- **Journal:** CEUR Workshop Proceedings 3191, 45–50 (2022), https://ceur-ws.org/Vol-3191/paper04.pdf
- **DOI:** none (CEUR assigns none)
- **arXiv:** arXiv:2205.14709
- **Database:** https://db2.fmi.uni-sofia.bg/3body123/
- **Fetched:** 27 Oct 2026
- **The paper's licence:** CC BY 4.0 (CEUR-WS Vol-3191's index: "This volume and its papers are published under the Creative Commons License Attribution 4.0 International (CC BY 4.0).")
- published by the authors; included as test data with citation; not ours
- **Files (124):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3body123/all_150_digits_123.txt` | `ccc21bd57438ced8b7f55ae5c89976e67dd7726623d8febe6070bbd2843c0869` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/all_150_digits_123.txt |
| `sofia_3body123/sol/sol1.txt` | `5288c2913fcfc4d9e7a409261693ec557b52324716a7151b906b690e7f307ff4` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol1.txt |
| `sofia_3body123/sol/sol2.txt` | `ecadedc635db8af7dc0fc0ba1435c5087b4a847b2c38c739475dbe129bbfeee6` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol2.txt |
| `sofia_3body123/sol/sol3.txt` | `072bd1d81b4aa8900b9a65ae1c5a578cb73d6dc08821855ff29623682c433c2e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol3.txt |
| `sofia_3body123/sol/sol4.txt` | `ba091148005fa4b24287729225083823f581ecfacffb7c635b37bc28aa6eea89` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol4.txt |
| `sofia_3body123/sol/sol5.txt` | `dba1813736ac187971a91e3de208460a8e1d6d026c0a5324c00edf078d97f201` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol5.txt |
| `sofia_3body123/sol/sol6.txt` | `2045d360976eaf4b0320ded884524934e3dd431f8c9b44789119543ead3919be` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol6.txt |
| `sofia_3body123/sol/sol7.txt` | `b600545c10286073bc85deda7b8ab8002083e4f204df6cf542a9b2c71b0dc31b` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol7.txt |
| `sofia_3body123/sol/sol8.txt` | `4772785928c9de2775456d6e5a9a537c1de51dc42be9b25430814786e63fe38e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol8.txt |
| `sofia_3body123/sol/sol9.txt` | `6d6a6f454540699edf59e2930c12c8d56ff48f584d555818c02afb10ef593cdc` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol9.txt |
| `sofia_3body123/sol/sol10.txt` | `c95c016fc7d58b001a013d8e9df123aea55f0ae9945e580100ee243bb0821d78` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol10.txt |
| `sofia_3body123/sol/sol11.txt` | `5aead81737b2789968d966df4ae41ee065f0cef839fed4e2d0805d2e83c0fe18` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol11.txt |
| `sofia_3body123/sol/sol12.txt` | `d0623735fb01063a79ab3f8eeb93323dcf185c0e548d187b08c426bbdf839f2f` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol12.txt |
| `sofia_3body123/sol/sol13.txt` | `79ca5678641378d2b5b0bbdea2ce5eb8c725edcdb5eff7974f251422f7a3e6e4` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol13.txt |
| `sofia_3body123/sol/sol14.txt` | `8de7edf2a686a7509291367828d4a7df47c19b3265bdc19e0aeef59f598a3145` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol14.txt |
| `sofia_3body123/sol/sol15.txt` | `b375ed02a7eb39e8d4a17eee17983f93645c08f3ddde96e679c1ff8bd5440baa` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol15.txt |
| `sofia_3body123/sol/sol16.txt` | `d9e6b94103fcacf6c4011389062104f6a52a885075f3fe4c6462b9e84a66e89f` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol16.txt |
| `sofia_3body123/sol/sol17.txt` | `ce0b0f2795aa4e1d02c0e81f0e984f9270ddd84f40b4b52188961a15b0b73d7b` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol17.txt |
| `sofia_3body123/sol/sol18.txt` | `0bb6288618c702842cdf0c9e9fc9f419de3d5f0e748a6c4e3de60d7a4dbb8744` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol18.txt |
| `sofia_3body123/sol/sol19.txt` | `2279d626912075f1f1dee1fabef0900b2a563813ad2bfa1d01c1e81179f9e083` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol19.txt |
| `sofia_3body123/sol/sol20.txt` | `a3fcca96d8c21cb92dec973ddb3cc6b1cbb0c6a79b38c94b02adf3ca39b61edc` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol20.txt |
| `sofia_3body123/sol/sol21.txt` | `041b8091e2c4531dd885ece6d6b14602d57bf410f5c24c9afdbc15566a3160cb` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol21.txt |
| `sofia_3body123/sol/sol22.txt` | `92cda43d44f22c18cc26e5e26a3a645bf6483a49f0f7664889f5bcbe6dbf7b89` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol22.txt |
| `sofia_3body123/sol/sol23.txt` | `f48b20cffd56bb349f550a0deaae3bb4b1b9ed4d0c8ea1e988d63f9c50bd28bb` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol23.txt |
| `sofia_3body123/sol/sol24.txt` | `5e97ce72c149630e161a49a3edb7c2d9ea1f3e63473022e414d4c65726730e47` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol24.txt |
| `sofia_3body123/sol/sol25.txt` | `e7e0afaec713d475a56db49aef71505131e64cc4d0b92e868d6d17cd917f8bf6` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol25.txt |
| `sofia_3body123/sol/sol26.txt` | `8e535ca133cbe30fc8954b58fc0d748b2e89372ffef86ece6c0b772ea4c19093` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol26.txt |
| `sofia_3body123/sol/sol27.txt` | `32099e7f3ee34f685b41aca127b0b3f90795ffffbd2b2d831f42a96ee0e39ab9` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol27.txt |
| `sofia_3body123/sol/sol28.txt` | `7b3cb99a4d4a6ccb33b0e3f34686b7fa2a7a662c4dcb2020a1e7d8f4c6a37583` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol28.txt |
| `sofia_3body123/sol/sol29.txt` | `f42ad72c9a21bb04cbc9b063d8a863db34516ed8bae6288cde259f0c618909fe` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol29.txt |
| `sofia_3body123/sol/sol30.txt` | `542c824c2db734608cee3bcd20c91e4723673cea49de7b66531b55fe99f6f198` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol30.txt |
| `sofia_3body123/sol/sol31.txt` | `897bb46eb1e50211fc768eae0f39ea4313404fe88d553914f25a1b3172364374` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol31.txt |
| `sofia_3body123/sol/sol32.txt` | `81b7d2aa215b6537149c1061b71761ca60290ced2eb75a14415147f757b2f1a1` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol32.txt |
| `sofia_3body123/sol/sol33.txt` | `423fedd68a6ea101bc7d47b910411e19d1601c0f8fa01d2ced4a7e5eadf9e2e2` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol33.txt |
| `sofia_3body123/sol/sol34.txt` | `5b6409d2e2b1f355042e32e34193bab5a114bc6a293290b3d19f0fdbabf941f4` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol34.txt |
| `sofia_3body123/sol/sol35.txt` | `3bc31a5a56c2f5afa48caf61ed5b3b527d305bad6c39ce64e5518ef506cb34df` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol35.txt |
| `sofia_3body123/sol/sol36.txt` | `2bac44d5cabef795977bafa7ad1fb04b362ba714f40bf396de6cc7507962eb31` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol36.txt |
| `sofia_3body123/sol/sol37.txt` | `fd65b147f651aab23276d0889f959675c236050b4d1b2e14dcbe67067c0635cf` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol37.txt |
| `sofia_3body123/sol/sol38.txt` | `46dcba3dfb82f573a0a61b100b0b4dd27071a7dbe4f8f0b799d87299a4d6a029` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol38.txt |
| `sofia_3body123/sol/sol39.txt` | `785d0b784d3bb64e8c73ad2b12b5cae2f88449ddfe96d8f856a1dc37d1f98dfe` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol39.txt |
| `sofia_3body123/sol/sol40.txt` | `9d1638ce982a72c567274bf75d62a7e1473aa079017e3d6adc20fdc6d52e9e34` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol40.txt |
| `sofia_3body123/sol/sol41.txt` | `cf1ae705026308165410f4197d305079feb69535fd7b182997d144edb6637de5` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol41.txt |
| `sofia_3body123/sol/sol42.txt` | `7adf68741dce638755fc124b2e57cf21d3db9b0d5939aa55b7ee72054b015947` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol42.txt |
| `sofia_3body123/sol/sol43.txt` | `8a7f067c11f48916d11f537c7cbf5525637eaaae85721f232d2b74b189e0e490` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol43.txt |
| `sofia_3body123/sol/sol44.txt` | `5022e32036241dcd8d108f53e18c19dcdf5ef0335c5c4c0f0bc91699a7f7ddbb` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol44.txt |
| `sofia_3body123/sol/sol45.txt` | `fd6164f834466832051297d69539b8a42229cd91a8f3710584cd4f082d8ae4da` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol45.txt |
| `sofia_3body123/sol/sol46.txt` | `7d77c4daec49c0cb90df6de50169f5a96591e8bcc1815415bc02949b856009e7` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol46.txt |
| `sofia_3body123/sol/sol47.txt` | `76febd897775909064d2f95277a8537ff3f946ab8c7e2d309d1c72a502851697` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol47.txt |
| `sofia_3body123/sol/sol48.txt` | `58d162b8fcd67382a41db69739f1c641e3559a93716e3f3696ef23c785d04def` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol48.txt |
| `sofia_3body123/sol/sol49.txt` | `53471fa647dbed426d279f7721fffc73845e7869e4d08b1ba104cc858ee3e7b3` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol49.txt |
| `sofia_3body123/sol/sol50.txt` | `10acb8601215fa45e66b6c3c706db2bbb2a6cdcfc1c106e5b9a0d22c74898a5a` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol50.txt |
| `sofia_3body123/sol/sol51.txt` | `c63b00f5ac523156c490df2ad73cd003795f9ab9cc070f48c0031c6375f06dd5` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol51.txt |
| `sofia_3body123/sol/sol52.txt` | `a7e812612340327c29285bf76216b744f2409abfa815d408c94996554765fee0` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol52.txt |
| `sofia_3body123/sol/sol53.txt` | `75d93fe4d796fafd52b6850190b0a61ef7ec925065d365366670417195cdc2bb` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol53.txt |
| `sofia_3body123/sol/sol54.txt` | `eaf7386b6f4057b1305d883bd55bbaf9fd6a05b006da8743bf36dfd18cee3239` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol54.txt |
| `sofia_3body123/sol/sol55.txt` | `c3c91e39a173d35f0b8deaed945c318185bc1a177eb72e64a35c657a92c95d82` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol55.txt |
| `sofia_3body123/sol/sol56.txt` | `829763b30cc5ca29f08c178a1f27ede7ce86b11473a4d047aab498ba2b00abfe` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol56.txt |
| `sofia_3body123/sol/sol57.txt` | `547b8bd7e0dfd4dbf85b4a8bbdad7f35baa4e8d07d803cf1119bfb5e6004fd0e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol57.txt |
| `sofia_3body123/sol/sol58.txt` | `f3b75c2e686b0296edeed21c340312f8c83a9759d6bc0b2af155597cbdfd3114` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol58.txt |
| `sofia_3body123/sol/sol59.txt` | `f67229a45ce0646fdc032b7dde09722eddb1f39d41acca961f798e684d3a903a` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol59.txt |
| `sofia_3body123/sol/sol60.txt` | `7ca6570424fa7c26dce9b8a8079e9281e978d9104d31bb30d7d9ab646f9d5c9a` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol60.txt |
| `sofia_3body123/sol/sol61.txt` | `28f3f73591c3f23b163a53ab244b8bdd3db9ff032427ce3e9c32cb30f2f3beda` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol61.txt |
| `sofia_3body123/sol/sol62.txt` | `c53a5deb963cca27590ca00053b1559f36dc9ec5fe0826054ae06a6f93fe8ceb` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol62.txt |
| `sofia_3body123/sol/sol63.txt` | `ec56a4a57b6860bc8e671751964ce7041bbbeccb66b01f2a2a9d2e90ebe95195` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol63.txt |
| `sofia_3body123/sol/sol64.txt` | `d1fbf7e50c0c4643263b4e26fb3a39c6edd66a6a5ea68cdc69624668a2da878d` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol64.txt |
| `sofia_3body123/sol/sol65.txt` | `a2ad0ccd202b68734574c4509038c713aa2791fd158eed991a2264d8357065af` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol65.txt |
| `sofia_3body123/sol/sol66.txt` | `702c578a32b90a21d7956a26dbbb20406081e74a4f59a7418ee10f56ecc95ac9` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol66.txt |
| `sofia_3body123/sol/sol67.txt` | `812616bfd5647a15b5d0ee27b21915dba527d634ebb1343647e6bad5b51193f6` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol67.txt |
| `sofia_3body123/sol/sol68.txt` | `5ffd568e35b158323c85226cf2fcf9a584d22708ba54a6d9a6c0561ad766b3d1` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol68.txt |
| `sofia_3body123/sol/sol69.txt` | `db0547184276c0bf58e8ece36ed535af8cbc5f5a70dde2ffc34d8109102aa073` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol69.txt |
| `sofia_3body123/sol/sol70.txt` | `26ed4237c5d849f67876889b8aa5dc9b30caf3be82b9e111cadf1296c683cf94` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol70.txt |
| `sofia_3body123/sol/sol71.txt` | `f24bf37222081d245bf97ee40565b2699a807f69b8d47413ceb0195f12b89178` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol71.txt |
| `sofia_3body123/sol/sol72.txt` | `165fe0d7aaa4c4131582d751440d71d89e9b5ddb2ae96c01d561407a52b227d6` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol72.txt |
| `sofia_3body123/sol/sol73.txt` | `59f3d1bed59e09d76308936a4ffb636c3ca1941989c145bf0aa7bb64c81b29c2` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol73.txt |
| `sofia_3body123/sol/sol74.txt` | `a1e643c8620bed7adc00c0dc3cd85cb40f9cbf53359c7ebe82707d0d57f6a23e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol74.txt |
| `sofia_3body123/sol/sol75.txt` | `7ee025ff33d9e1fffae4d5bb38ae00eb3eb45056a37356edb92442dabb050cab` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol75.txt |
| `sofia_3body123/sol/sol76.txt` | `45242da9fdd824d07e5335861c6e78f8d167191ce24ba634472842a2f7d3381f` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol76.txt |
| `sofia_3body123/sol/sol77.txt` | `35501b5c1630279b0f5b1faaf312c547895f9e0565f7a1f4ba669942e9d8f52d` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol77.txt |
| `sofia_3body123/sol/sol78.txt` | `e0e741b721d86e88a979d1b0585aca99afa8fac2f896d406342e4ecf2c0a29ba` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol78.txt |
| `sofia_3body123/sol/sol79.txt` | `64eeff175872b9586c9306f50b2da892f3ffe79014e97ac10f9b375701705c76` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol79.txt |
| `sofia_3body123/sol/sol80.txt` | `4177ac1841518129f125232e972c61a114aec66c78cb663aea97fb385f4c0152` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol80.txt |
| `sofia_3body123/sol/sol81.txt` | `b2ef7e0853204e5bf03f27e14b8234ca5dc401aecdfdfc70d062a77620006ae7` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol81.txt |
| `sofia_3body123/sol/sol82.txt` | `767b329583f8c364ca40b56caa0a6369553fc9720ee0dc5f349bde2a37cb4360` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol82.txt |
| `sofia_3body123/sol/sol83.txt` | `6e9049a52b9f6f4bd1549481d46f02596831a43222c8758d1b4c6b86edbfd0ff` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol83.txt |
| `sofia_3body123/sol/sol84.txt` | `f30d58e090e7214bb103eb09ddcc564b9d2363a662491d02e08ae65d1cdac2e0` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol84.txt |
| `sofia_3body123/sol/sol85.txt` | `b60addf4d9342aab2d407ff8099882f5ddf0eff03c7341aeba2f4df38292f910` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol85.txt |
| `sofia_3body123/sol/sol86.txt` | `8fc22b70d415f85a6de9a7c58aabde601632b51c9700813a15888ac6d2ef1727` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol86.txt |
| `sofia_3body123/sol/sol87.txt` | `3452f4b319104821243cb4072a03bd62ac8bad31a1d8d2b3b3b38df305a73b62` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol87.txt |
| `sofia_3body123/sol/sol88.txt` | `96c95172a8c1569c96adf36936ad7effa040c257deebc3ad4e80c751696118bc` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol88.txt |
| `sofia_3body123/sol/sol89.txt` | `c9a22dec32ce6870da53aa4296d7845d9bfd5a74f005bbec37a4e5f8b180cf5c` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol89.txt |
| `sofia_3body123/sol/sol90.txt` | `102e090cf95a57a601ae777be84c2fc8cdb46c336af4a0d44d2bf14860d58ebc` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol90.txt |
| `sofia_3body123/sol/sol91.txt` | `8548127d4606ff81ef8ca2981838052a410acc1dc09dc1fe247f225e4f81f9ef` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol91.txt |
| `sofia_3body123/sol/sol92.txt` | `016ef082335217dccc35395a7da0487a5f85550276492b8d772122a53a990296` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol92.txt |
| `sofia_3body123/sol/sol93.txt` | `f3482077fd0a52fcdabf2c7374469d160d1beb9185a2bfde5d09068541c7cdce` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol93.txt |
| `sofia_3body123/sol/sol94.txt` | `f21d8e6fac23312423e8788ed8d54d3c5216256e8f21b55a7f6ac83ed65933f9` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol94.txt |
| `sofia_3body123/sol/sol95.txt` | `d51affca7b86850dd3f41a070d6992c7ee354807e02b056f45fad2f1de27e413` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol95.txt |
| `sofia_3body123/sol/sol96.txt` | `6f95a8200f2cdd701c4e2d74f8fbff32c4ac6cd4ad0b96fe3114283d9578d6cd` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol96.txt |
| `sofia_3body123/sol/sol97.txt` | `3baf1335b564ff63fadcee7de345ca450301eeb2c33223a939f9be5f8f24decd` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol97.txt |
| `sofia_3body123/sol/sol98.txt` | `a66b3598988533fff2f8ce17fe95b64e63a2df24b110e7c580ddcf09569e95ea` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol98.txt |
| `sofia_3body123/sol/sol99.txt` | `dde39c9117d57bdf63d977dfcf0e3a7bed8d243edc3162e8e72aa7f3795a032b` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol99.txt |
| `sofia_3body123/sol/sol100.txt` | `bd205a666823c16d53d556ce09a16c16372a6fa1e2b77f229d1430cd05531aa5` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol100.txt |
| `sofia_3body123/sol/sol101.txt` | `2ed20fd0a27acdb6416509cba3cf79054718cee71c7c620b4cef312928bdce61` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol101.txt |
| `sofia_3body123/sol/sol102.txt` | `06be70511b313475de8f99bad3e0d7cba217f650a50dd96fff5d266686eeeda4` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol102.txt |
| `sofia_3body123/sol/sol103.txt` | `353515e09e15f92aa23f20814fdb325529efcc5945a95c4bb52fb7dd070e471c` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol103.txt |
| `sofia_3body123/sol/sol104.txt` | `3880995aef6314f9929b192b709f0535d38896cc14911e40c23a5c0f8ad0998f` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol104.txt |
| `sofia_3body123/sol/sol105.txt` | `6f67d3aea6b7d83f815838a85538e5782b8b6ede6586772c4b4e024440310c30` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol105.txt |
| `sofia_3body123/sol/sol106.txt` | `b76c1991dfa0a212ef3cbbc616490a2c77d579d031b07453f88b58cb0963c5a4` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol106.txt |
| `sofia_3body123/sol/sol107.txt` | `1d85a0dad80c2bb471c8f607e1af4ebfd55d7369583a048b31ded93b97a8882c` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol107.txt |
| `sofia_3body123/sol/sol108.txt` | `89c8c654a997f77029fcde902f775227920e3990e87fbb5660b9a97ecbb20950` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol108.txt |
| `sofia_3body123/sol/sol109.txt` | `f4da2a6e559e069e7df23ddd1051d961cb6713160c2c4f090c42f243195a0fbf` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol109.txt |
| `sofia_3body123/sol/sol110.txt` | `96a0714baa30940a4e89bea07f3c4eec50de4470a28a3ddad05dd5d2bb50bd83` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol110.txt |
| `sofia_3body123/sol/sol111.txt` | `8fab1944fb79d5090e2c8e15bdc423fdd78c0f562c5bb763dde7796996acca4e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol111.txt |
| `sofia_3body123/sol/sol112.txt` | `e019582b5fb7b4e7a7a4454fa25662c18a655fb9f1c0df709cc4da04ce4c825a` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol112.txt |
| `sofia_3body123/sol/sol113.txt` | `0888f41ed61a3a4eb6bd1f1f32339f04611045d9b6e5e02bb095020ecf34a249` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol113.txt |
| `sofia_3body123/sol/sol114.txt` | `5969f369cf87c559228e0d38a3ad2cb3a4df393c354a04487771fb0fe1867c41` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol114.txt |
| `sofia_3body123/sol/sol115.txt` | `e325c7acbade0f24c3652a28369c2609a9d0804fbba96fdf384d6b8749e24137` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol115.txt |
| `sofia_3body123/sol/sol116.txt` | `fbe13d1f5976c3ca147b9184fea8af46c6557ac98288e304f3314ad287d8cfd3` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol116.txt |
| `sofia_3body123/sol/sol117.txt` | `52f1a6817e546881cc5665db158da53df2eb84b7629003b1257f8e3f14b90cd1` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol117.txt |
| `sofia_3body123/sol/sol118.txt` | `138b95445e5cbb89378d8a2280664de273da4ebea168c26446ff6843a30ef0c7` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol118.txt |
| `sofia_3body123/sol/sol119.txt` | `59c91f382866804227b440a3bebb9ce4010a7b3eaee0d2b2af73fd960c023cc6` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol119.txt |
| `sofia_3body123/sol/sol120.txt` | `e8e82bd5a0f5fb4d709ba52912e416b9d0bc126b35b7b704d24ce67f371f461a` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol120.txt |
| `sofia_3body123/sol/sol121.txt` | `1b66302adf00b61c77fe809d0e85f6e185a5cf878d78f0abc21e67b65afabdc5` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol121.txt |
| `sofia_3body123/sol/sol122.txt` | `9f8f34802193347864b5c98d6875e9d6c0bb27f022093ac2e515627a5355201e` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol122.txt |
| `sofia_3body123/sol/sol123.txt` | `9c4bee5357f43bac7d002656e3074baca8ccdf0dbc80558d99a5d0c869867f0d` | https://db2.fmi.uni-sofia.bg/3body123/datafiles/sol123.txt |

## hristov2023

- **Authors:** Ivan Hristov, Radoslava Hristova, Igor Puzynin, Taisya Puzynina, Zarif Sharipov, Zafar Tukhliev
- **Title:** *A database of high precision trivial choreographies for the planar three-body problem*
- **Journal:** Lecture Notes in Computer Science 13858 (Numerical Methods and Applications), 171–180 (2023)
- **DOI:** https://doi.org/10.1007/978-3-031-32412-3_15
- **arXiv:** arXiv:2210.00594
- **Database:** https://db2.fmi.uni-sofia.bg/3bodychor462/
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated as open: Crossref lists only Springer Nature's text-and-data-mining licence; the arXiv copy carries arXiv's non-exclusive distribution licence
- published by the authors; included as test data with citation; not ours
- **Files (463):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodychor462/digits180.txt` | `317fccc0c1be53d406f28b3e757754618d03177cbbe36347856758326b0f1503` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/digits180.txt |
| `sofia_3bodychor462/sol/sol1.txt` | `257b98d8e7f0e8f3ec47e9c5105c1de8b67572a3b855ca902bcb4f81b833e60e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol1.txt |
| `sofia_3bodychor462/sol/sol2.txt` | `0631d4972c0280a4466a522fae3f51303e93dc319e9164a34d4d266470aa3e5f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol2.txt |
| `sofia_3bodychor462/sol/sol3.txt` | `ce2eaecf9e5a9ddb2480019dfd14b0bc99cfd2421fd40f857def2a5dd466c776` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol3.txt |
| `sofia_3bodychor462/sol/sol4.txt` | `ca4e5ae222c02022802306b868e8b5d68b8c7f2d8c1c8964cef46e26b82b7035` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol4.txt |
| `sofia_3bodychor462/sol/sol5.txt` | `1536267bd6572a14a3ea1230a30406d71f4ca392b1a500dd9e0845f90666eaab` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol5.txt |
| `sofia_3bodychor462/sol/sol6.txt` | `1f72b4fc90c14b67a78a9899dbf621d75e383dc9cce0144cef6fbd6c172d9294` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol6.txt |
| `sofia_3bodychor462/sol/sol7.txt` | `4f1cd976dd8d96dab28cd2c349a9ed8de754ce273e05e4dec7b7132bffa9d633` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol7.txt |
| `sofia_3bodychor462/sol/sol8.txt` | `2230b610ae6f56d91775f4ed7ec82f4cf4fee9c44b062b575d0f9b8a514f996c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol8.txt |
| `sofia_3bodychor462/sol/sol9.txt` | `f1fa5ef857f04265138321e988c0642ac88b9569271d781da9f2868d915a482f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol9.txt |
| `sofia_3bodychor462/sol/sol10.txt` | `70a43731557740747c58d3004a47f5b4de794331df46c8d2d28a07027ff4d7a1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol10.txt |
| `sofia_3bodychor462/sol/sol11.txt` | `f1f1a4824a203ac96b4bf8c5cc44dd3727f95586203f230af1e5b1621b3540cb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol11.txt |
| `sofia_3bodychor462/sol/sol12.txt` | `b8bea18abcd9ffaff67ce55ada6f43e756597d7805c593579453a8096c939ccc` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol12.txt |
| `sofia_3bodychor462/sol/sol13.txt` | `a6a695674d09d52cd7030625ed2eb2fa6c46447c435d0a49defddc5b4986b8e9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol13.txt |
| `sofia_3bodychor462/sol/sol14.txt` | `9a4c5ac6c36075f296b65d63695223c340288471179d71cafbc3d570877cfb05` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol14.txt |
| `sofia_3bodychor462/sol/sol15.txt` | `02c2225698822a9da7e19c591cacd631b446af7ce80aa6c591050fb34b70a360` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol15.txt |
| `sofia_3bodychor462/sol/sol16.txt` | `b35572f2f86f90b922310ac05a9a353abfbd43304d058e92afa852aff0b136ca` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol16.txt |
| `sofia_3bodychor462/sol/sol17.txt` | `ea75c38982137d0a98104761b98b67b6e779476fb84e755526172d09591e72eb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol17.txt |
| `sofia_3bodychor462/sol/sol18.txt` | `5100320eb41d5ba33866bb9c2b0ff497d1af8618945d18a48e7b946f317df60c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol18.txt |
| `sofia_3bodychor462/sol/sol19.txt` | `ff90a9e0f816ac45048e2e1c63ac8e91daa1765788a7ef11ef559c94c59201da` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol19.txt |
| `sofia_3bodychor462/sol/sol20.txt` | `db740f680dfc4c2a7f760f9971c282d3e4fe5db75ea4e17bdc56fce823b7b611` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol20.txt |
| `sofia_3bodychor462/sol/sol21.txt` | `9ad9b7d9b82ae3ec76aca35fb0628d39ea3aafdb7246cf64b0d7c8ef2b30586c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol21.txt |
| `sofia_3bodychor462/sol/sol22.txt` | `6a726e00e2a31de055cf40b030ce2f87f8c9ae9446c7d9540e356c66295d060e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol22.txt |
| `sofia_3bodychor462/sol/sol23.txt` | `60058e1c972236e3892b78af664d79f493b386f0eacd3426332fd07548d32924` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol23.txt |
| `sofia_3bodychor462/sol/sol24.txt` | `f0cdc4838f192e83230197dbf7dbafb15d6b448ccba6cf9d554b0896c2a5abf1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol24.txt |
| `sofia_3bodychor462/sol/sol25.txt` | `89a91517694af5be190e7092af52946ef135335c18dd021949cb486711ee3f92` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol25.txt |
| `sofia_3bodychor462/sol/sol26.txt` | `d0e015a2d5f4b6628e4dd385d54f53b74fa14469bb21dd0d0e2faeeb47bc90f3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol26.txt |
| `sofia_3bodychor462/sol/sol27.txt` | `50e2db9e9f8e0fc8164eb65b694a0953660d9dd0a9257ae49229fdc097c3c700` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol27.txt |
| `sofia_3bodychor462/sol/sol28.txt` | `d5a187fee19a0850ccf89e2a3878a6278f8f13dcfc5c8ae1d2a686f9d02ee110` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol28.txt |
| `sofia_3bodychor462/sol/sol29.txt` | `fd3643e0854082dbae2e871bc5c1ea52f0ff12239a4d5c34d45ea3cc16bd4ba7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol29.txt |
| `sofia_3bodychor462/sol/sol30.txt` | `0cc359056dcd75e2cc1dffc059371af8617df2f0b42bc83dbdb7f486576ef0f4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol30.txt |
| `sofia_3bodychor462/sol/sol31.txt` | `e348c603a3654869a2d42e4cace999d389c3baefd2082a3052e6a44f6fee444a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol31.txt |
| `sofia_3bodychor462/sol/sol32.txt` | `bfd0fa20e6bd2621ff86956165613224f99b2f62d2a77fa7193b4b00574c4397` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol32.txt |
| `sofia_3bodychor462/sol/sol33.txt` | `798b42857ae1d19466960cac4303d9112227b6b002abb24bb688e07480e26d92` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol33.txt |
| `sofia_3bodychor462/sol/sol34.txt` | `0b4697360754162cd6cfd85361c91162e28f6fc5f42a1a762ff669a1d3972a6b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol34.txt |
| `sofia_3bodychor462/sol/sol35.txt` | `4d7f88961cf2b724ff28f472fabae94deff30d8ced5d7974989a4fb238496421` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol35.txt |
| `sofia_3bodychor462/sol/sol36.txt` | `5a26dd1dc47a34751152c7747eb7ad8b7ae51c61328428f0f492b78fcd8d5bd9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol36.txt |
| `sofia_3bodychor462/sol/sol37.txt` | `fa56c0903ba414dc0bc5a43d3db124008c09b6d47b25bdc7af7aa628acc1b9f5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol37.txt |
| `sofia_3bodychor462/sol/sol38.txt` | `fb71cd044a11efa7b0655c70ff9f7de2e98ad482b899be0bb37ff2959f46b7a4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol38.txt |
| `sofia_3bodychor462/sol/sol39.txt` | `ad34b23a4b4aed74c57115c830f589e7ac52b1ae955283f2d611143594a7f16a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol39.txt |
| `sofia_3bodychor462/sol/sol40.txt` | `fee81a5e2d1396462b5aab755e9fc36322986ef93dc160555103e3cb61ae5ba2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol40.txt |
| `sofia_3bodychor462/sol/sol41.txt` | `b686d52d956ebdbce232d7536348e2b55f4018c4a17ae360e2ddd51e001d4961` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol41.txt |
| `sofia_3bodychor462/sol/sol42.txt` | `0ad2050ca4b5d67860d4e9f47b89080ea354761df0d1aa7425bfc10925fc2d60` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol42.txt |
| `sofia_3bodychor462/sol/sol43.txt` | `6a5d48bc19df95075b73ef165267dcf2c19414fd85f0ddbffd747678cb483133` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol43.txt |
| `sofia_3bodychor462/sol/sol44.txt` | `22b68a44f08687e87ca2c8ed586f03330af14dee99746d31e8930de3a61d8d60` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol44.txt |
| `sofia_3bodychor462/sol/sol45.txt` | `b24bc9646d85694dd535a4e7599764c460c3e637079f51e9ce77e60d652734ee` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol45.txt |
| `sofia_3bodychor462/sol/sol46.txt` | `41fb582a4d2297cdec45d5700f9d1b0d727c3054f79ff800b7ebd92738ef509f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol46.txt |
| `sofia_3bodychor462/sol/sol47.txt` | `b82d9addf70e4ef988270e2a879653e41a6fdf3949da9524c155b05f6f549601` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol47.txt |
| `sofia_3bodychor462/sol/sol48.txt` | `b76e296908be2befba9f9a4c5bf27a1b002593b9cf73db4aacbfec7871efe0f0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol48.txt |
| `sofia_3bodychor462/sol/sol49.txt` | `1211c65d5081fe29aaf552c7cc68b3d81803c37ff80025bdfd1d81506dff4fc4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol49.txt |
| `sofia_3bodychor462/sol/sol50.txt` | `5a8dad55cbdecbff7ff1fc98d17eb742f11318b7a36df757309aa92d486ff4b6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol50.txt |
| `sofia_3bodychor462/sol/sol51.txt` | `45aaa2bff122c9e49b9a21318a56097b2ae5b42d9f87da3db60514ff55ad21be` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol51.txt |
| `sofia_3bodychor462/sol/sol52.txt` | `90b2473d893697a43fd76298a0ec4ab73114a05077a0103a9aa18fa8ee7d5738` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol52.txt |
| `sofia_3bodychor462/sol/sol53.txt` | `95f3680a2f71103a1b0c2f03793606ac4d1ced34998dceb6d877cb68105d2cf0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol53.txt |
| `sofia_3bodychor462/sol/sol54.txt` | `53d52f9f0de2be79cf22d1a8f350a0b87fecd01c4c4fc8381c9e2cb752f898c7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol54.txt |
| `sofia_3bodychor462/sol/sol55.txt` | `d82a3e53a38e4c98fcaae0898fa7bcc9f701666e4e2a2d2408cb98a048d45792` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol55.txt |
| `sofia_3bodychor462/sol/sol56.txt` | `aa0ba69265656361c595fbdaf70e39d65ef1f2c9b888c2196909d6b757774b87` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol56.txt |
| `sofia_3bodychor462/sol/sol57.txt` | `a01f087bfb5d8394cbb6c5a73abebc71aaaa81a6d1ba139a96b5ef51f0f819b0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol57.txt |
| `sofia_3bodychor462/sol/sol58.txt` | `5c58407f7fff273cf9f4866e4fecd990f57a499d90b8cb5388093514d65efa59` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol58.txt |
| `sofia_3bodychor462/sol/sol59.txt` | `28c164e34d28222e9abf433688479df5388286d51d2b40bfc5082827c6b21b9a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol59.txt |
| `sofia_3bodychor462/sol/sol60.txt` | `589da09fd3bf91ceef60529ed3a497c2524d1cff11c44183ad1627f5fceed09f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol60.txt |
| `sofia_3bodychor462/sol/sol61.txt` | `8670f1ad5c6073abb4cbd598daf5f7f8ce1e82ef36090e4299e45bcecc25d054` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol61.txt |
| `sofia_3bodychor462/sol/sol62.txt` | `304d4afaeafd2733582d26f11dbe231b4cc48d95d5b540417fc551810e841ba5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol62.txt |
| `sofia_3bodychor462/sol/sol63.txt` | `6974410a8be5e332687aaac5b109ea1e46562789dd8e00461cbec21868bde7c0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol63.txt |
| `sofia_3bodychor462/sol/sol64.txt` | `8772ca980fee6043d9a0c59e15fdabb6280f0120a6b1ac82f15b8f405e684a08` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol64.txt |
| `sofia_3bodychor462/sol/sol65.txt` | `bff8cb18faec7e6e49ace23995b9c35742ce73504dea67fc2a67bdc7170f134b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol65.txt |
| `sofia_3bodychor462/sol/sol66.txt` | `8d0263332698a50b40ddc1b590a2e0db911d2b90bad02ddeecf1c4bd846412d5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol66.txt |
| `sofia_3bodychor462/sol/sol67.txt` | `1d30ee4b2bc217809f9ea092d8b67e824f6178f5425b8af2cf1377182b1db797` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol67.txt |
| `sofia_3bodychor462/sol/sol68.txt` | `90b07020836b05495af0948ee40a9115191a9abdb849b95cae4c19fc7379c202` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol68.txt |
| `sofia_3bodychor462/sol/sol69.txt` | `f68d50ed1387c0df1a179bf75b9b3e142fc2c603e3fbc71737b099632b90cfb5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol69.txt |
| `sofia_3bodychor462/sol/sol70.txt` | `f281878330519f02d15f4a4d764983f2fba7aa818a6536e995140c91e6add1a2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol70.txt |
| `sofia_3bodychor462/sol/sol71.txt` | `11149fbc6ad3cedb0b0936dd611ee7c1cf51170c2127f9463933ae1fa2d0b3ec` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol71.txt |
| `sofia_3bodychor462/sol/sol72.txt` | `c9057fbdaa5fe93d109f9ac8c6e88e9f5b9039ffe4372bb881d6a5b20646e1fc` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol72.txt |
| `sofia_3bodychor462/sol/sol73.txt` | `75a704812039d150cceec8673962c340cb2f2f061ea6e7232a548009941c2478` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol73.txt |
| `sofia_3bodychor462/sol/sol74.txt` | `d66900f0642816414e130081b841e41ca8662536f8a6e690334eb34a9e2ba8c8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol74.txt |
| `sofia_3bodychor462/sol/sol75.txt` | `d425502646bd8d27fc52bc7b8428dd5114175dec79915c1d1dfaccd7bdf1a335` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol75.txt |
| `sofia_3bodychor462/sol/sol76.txt` | `374464b8a593d63bfc53f43ded7ca3471436a957e02ff656b279651a8a438eb7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol76.txt |
| `sofia_3bodychor462/sol/sol77.txt` | `d69cb051ea907e623264e4f26e0eef2649029acd969287e57e47d51e8d907bd9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol77.txt |
| `sofia_3bodychor462/sol/sol78.txt` | `81249094ed0657ec1c8e77f4493991c5ee3e75b5b4513d0b438100d76824d4e5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol78.txt |
| `sofia_3bodychor462/sol/sol79.txt` | `debc5fd2ddecf88424fcbb14f091b70f1e642a31bcef444ae283cc73e3baf077` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol79.txt |
| `sofia_3bodychor462/sol/sol80.txt` | `3508a4243673a9aa8496a093ad79f63b8873b2a549702f1a42980d7c51ce8dc5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol80.txt |
| `sofia_3bodychor462/sol/sol81.txt` | `465cb2c9d78cd3d5b462472bd971fe850842b5daebb12bd07400ac1421d4137d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol81.txt |
| `sofia_3bodychor462/sol/sol82.txt` | `ad5e2c56cf61f39f5d536b9b9bc65b20575b85f3aeb5d01ee09cd186dcc6dd6a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol82.txt |
| `sofia_3bodychor462/sol/sol83.txt` | `7e52de2161eb39434c5a4b49308d72632ce15512ba58e773db85b4dc61fba738` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol83.txt |
| `sofia_3bodychor462/sol/sol84.txt` | `d8b48841aa9699a5de13466f43ea2b0abfb88771ceefc75eb84b7481ab3189b5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol84.txt |
| `sofia_3bodychor462/sol/sol85.txt` | `90d7a80f4cb924da1d93df205b05fe097baeba6ae5a3d5d9a47d8556e5d7770d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol85.txt |
| `sofia_3bodychor462/sol/sol86.txt` | `05852f51910119823459444e0903415e2c2ecd05a29746f57bf55546d95d7eae` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol86.txt |
| `sofia_3bodychor462/sol/sol87.txt` | `56e45dbc95ea1ef87a6d0012ca116cd183fc5f52c336f99136512a904feb553d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol87.txt |
| `sofia_3bodychor462/sol/sol88.txt` | `4fad77caae6499b623675ad55ac412b86ff36351eb187603af7c2678922d2e6b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol88.txt |
| `sofia_3bodychor462/sol/sol89.txt` | `1b4fc68daca9fced31507a77e98294e7ca1e216872891a6a5610eecbb36b5d26` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol89.txt |
| `sofia_3bodychor462/sol/sol90.txt` | `f9ccc50d9d8121cb97b931c57c03e355784c8476ffccb4fe4123eea4139998cb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol90.txt |
| `sofia_3bodychor462/sol/sol91.txt` | `5fa11c33715f61155f37b320bc564c879ad6d1f79672b13a30195b7ea5345e7b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol91.txt |
| `sofia_3bodychor462/sol/sol92.txt` | `b1b0027e0cadf298a32fd45af24654c9ab120dc0399271b44cea980b0401acbf` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol92.txt |
| `sofia_3bodychor462/sol/sol93.txt` | `b1548463d2f9f046704d84170c99c60b8f7916f27f965387dd7a0bc8fad0522b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol93.txt |
| `sofia_3bodychor462/sol/sol94.txt` | `b776fa38a5aa4bb49cf958c790528321a127d3961987812ee7e5b4d84f304927` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol94.txt |
| `sofia_3bodychor462/sol/sol95.txt` | `d1973c050790a156c01af20282db555019840117ab330ce242fe4b975471b69e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol95.txt |
| `sofia_3bodychor462/sol/sol96.txt` | `3221603460bfa9cbbbb8fb3b0f28d105687abe8c0f868982eded0ae59032a97a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol96.txt |
| `sofia_3bodychor462/sol/sol97.txt` | `e4eeaef3d5fc2189e98f2c5fc631dfd71a44b8a10e6ac62f39ab9955635f5179` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol97.txt |
| `sofia_3bodychor462/sol/sol98.txt` | `641024bd25e4ea6887c5c3dcdbef8676fd963e1e762e615521e31964034f0fb5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol98.txt |
| `sofia_3bodychor462/sol/sol99.txt` | `1638d494c1bc839f6dc9d2960b81a27087b4d638097d24f0220f24e02afa47c9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol99.txt |
| `sofia_3bodychor462/sol/sol100.txt` | `14d08d040a6559ebaf6350d0e93d915d3cf1d3bdc78e649d55dddf5843653119` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol100.txt |
| `sofia_3bodychor462/sol/sol101.txt` | `54e779e3e001362f97ebae348aea77b2bc090aa0503da86bb40dadec9f6089a7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol101.txt |
| `sofia_3bodychor462/sol/sol102.txt` | `477e9b74f182fcd73627141022aa4daaea01024acd97329927660871d39d83e1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol102.txt |
| `sofia_3bodychor462/sol/sol103.txt` | `b08688dbffde19336603ea6ca8d3c41fb61e63f904bfa61af686d0ec2c95e185` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol103.txt |
| `sofia_3bodychor462/sol/sol104.txt` | `6f859b2b9b1f25e45531e9f30453fcd26eb97a758148fcc863917601b459e79b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol104.txt |
| `sofia_3bodychor462/sol/sol105.txt` | `4e7fc2d6eacee63e4b67099e32b26bef2c2f0b870f2f55aaa01b663953430c6a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol105.txt |
| `sofia_3bodychor462/sol/sol106.txt` | `6a7a71837ed42643a61457066f31ec152d74b7ea7b9c1049f0449f9d9764b608` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol106.txt |
| `sofia_3bodychor462/sol/sol107.txt` | `8658903ec59a0bcbce6b2f4c163df79dc8dfe8fcae96f84947bf9b30f87621f3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol107.txt |
| `sofia_3bodychor462/sol/sol108.txt` | `2fa236c3dafeccc1c7cff768fbb8d9a5cb4bd1df3323f684762847e456a13dff` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol108.txt |
| `sofia_3bodychor462/sol/sol109.txt` | `da7cd369836cd48d8ad273b4ad928ba5f28b144831a8c531f8678d4e9abe3106` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol109.txt |
| `sofia_3bodychor462/sol/sol110.txt` | `ec413428994886b30c2b1b1b0fc11052af669f013b64ed04eddc8e6307df8516` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol110.txt |
| `sofia_3bodychor462/sol/sol111.txt` | `d600ef9802a1694d8a859d9a75ba0d1a001df57e4acbc2e3392cb7c752a5b681` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol111.txt |
| `sofia_3bodychor462/sol/sol112.txt` | `7d5f3b38c8c3319eadc2ddd89ddb1f66fecf3669f3fa24581a2bf38d9d5bbf3f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol112.txt |
| `sofia_3bodychor462/sol/sol113.txt` | `082aa3a74674bf60742e752720f9eacc42a51b27c4ff859836e13640e00f9a5a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol113.txt |
| `sofia_3bodychor462/sol/sol114.txt` | `1ecf6227f749c52e22eb55284a1810d19aaf90f56df74718e5e91ca882fb1347` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol114.txt |
| `sofia_3bodychor462/sol/sol115.txt` | `3ca0aed594a7a710fdbd2f9248cd27b9cabd724801bbc1e343454a4f406a4e3c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol115.txt |
| `sofia_3bodychor462/sol/sol116.txt` | `85d7133f163275c26e65f0d6cd483ca87e6000b67f5890a3f9bac66d419e64ac` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol116.txt |
| `sofia_3bodychor462/sol/sol117.txt` | `47ad3d3166e255a1bc4256f2ed3f68345662e877a98a2df81b268aa3d9acfe27` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol117.txt |
| `sofia_3bodychor462/sol/sol118.txt` | `1e198ed9dc583346a6108a180f5491cfcae37fbb4f078ade95a82fb0aba6ca0e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol118.txt |
| `sofia_3bodychor462/sol/sol119.txt` | `5a5082bdddb2e3abea296181a7b0dda6d996e18a23d54be9cf9d2ece39cc0ce0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol119.txt |
| `sofia_3bodychor462/sol/sol120.txt` | `78f9933e039dea897ddaa7bb6d84f31c72f1797a9ef8a3fd364722b99a4c8d5f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol120.txt |
| `sofia_3bodychor462/sol/sol121.txt` | `4239262061430e062767ad1e96b93e2e6dbeb459a2ce8b76f955f9e0f912f1a0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol121.txt |
| `sofia_3bodychor462/sol/sol122.txt` | `87d4ab66065f9e92be7af24abf8efd34a2736c2329018dd59bb22aa68c05dd33` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol122.txt |
| `sofia_3bodychor462/sol/sol123.txt` | `3f0e4319d42858b9ce526163c8927ce5d9b17b04c53f1e0cbca57814b73b6342` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol123.txt |
| `sofia_3bodychor462/sol/sol124.txt` | `911bacca7d858d4d3b4705fc45f0baf8641016ca47844c3da4f451b73c717451` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol124.txt |
| `sofia_3bodychor462/sol/sol125.txt` | `970e1713d7b2ae4dc9dc6aa9934afbbca133ae6a3391123f0d252ed09cf65b0a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol125.txt |
| `sofia_3bodychor462/sol/sol126.txt` | `78141d2ba190444d34f5dc24a260c270c9881b030191af082fd4b9d2439a5c78` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol126.txt |
| `sofia_3bodychor462/sol/sol127.txt` | `5b65dbd305f75163e67c82aa9c0eb113717051def142175dfbcc909160209218` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol127.txt |
| `sofia_3bodychor462/sol/sol128.txt` | `a4e79d9087897ac2006ad810c20abd485aa429f2d0f3d35d3a6be459c5f100d1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol128.txt |
| `sofia_3bodychor462/sol/sol129.txt` | `4a775ea1122480acdb9405c80e9ee251d36cd2c809df65e8df79af3ab42512c5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol129.txt |
| `sofia_3bodychor462/sol/sol130.txt` | `7bf35b85f6c98cd27eec4cd8b000d2df2e0d25187c52755f1587837888d308e6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol130.txt |
| `sofia_3bodychor462/sol/sol131.txt` | `d50d5a309644de54a18f45dbc1721d6d62e4472a3d4f58de1741ede9f24989ca` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol131.txt |
| `sofia_3bodychor462/sol/sol132.txt` | `23458dc5d46ed62e585fafd7d83cfa1dc0ef79e071a16cee766a75ba868b9d8e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol132.txt |
| `sofia_3bodychor462/sol/sol133.txt` | `bea319e2d2a76efcb86f9b50f2b25648546d776e3a937ed69f7312a0ff94b554` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol133.txt |
| `sofia_3bodychor462/sol/sol134.txt` | `f0c8c3650213363821fd3945ed4942f5cfe8b430c32539f4ef2ce0d91963e0d0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol134.txt |
| `sofia_3bodychor462/sol/sol135.txt` | `15ae5bc7393cbc384bcc09f182833810d9dccaf69b516ed554b6765a5e77fda6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol135.txt |
| `sofia_3bodychor462/sol/sol136.txt` | `26bd84224f8f3d83319ebd9acbd7e7ec23650d0a212d9734939a6ced428a2e01` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol136.txt |
| `sofia_3bodychor462/sol/sol137.txt` | `9656a01cf71a285198fdde6b5110d98025b3aa90af86ab823fd56e5ee9b7b91b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol137.txt |
| `sofia_3bodychor462/sol/sol138.txt` | `83f16b1227fb2502c308b792904b264031770b70c02b36a9ee65ab6afa2aa33b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol138.txt |
| `sofia_3bodychor462/sol/sol139.txt` | `01cf7afa4d94f20fd4442d54218a5849adc0851909e5a4ca51895619f8bbe566` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol139.txt |
| `sofia_3bodychor462/sol/sol140.txt` | `2721cc0a3a698ddc44f63cd2fcb4445fd8580c20f0f9b7db526faa520f951c57` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol140.txt |
| `sofia_3bodychor462/sol/sol141.txt` | `e49ad24c3858d005a2ec4e585a12cfbcec6660611166150e4938d2f08ab633f7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol141.txt |
| `sofia_3bodychor462/sol/sol142.txt` | `693810accf8f91358755fd296a716dd79ee33e26ec03f0c717caf1b8825d1f35` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol142.txt |
| `sofia_3bodychor462/sol/sol143.txt` | `f7e93b0bf65c509f972be81f75f413ba86e732a888bdaf126c6867c94e2dffb3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol143.txt |
| `sofia_3bodychor462/sol/sol144.txt` | `0fbd08dc66b56d66852ff5467ed6d5cfe6e84c43fbef7dc91f1b96d94e627d96` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol144.txt |
| `sofia_3bodychor462/sol/sol145.txt` | `ebd3711e411ef86875918f37fb28d250de1256e9ae31f87e7d610dcd5919951e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol145.txt |
| `sofia_3bodychor462/sol/sol146.txt` | `e8459eceedfae2159c88d39ebcc4ae52fe16bd73d52a1006cc7109be2bc5ccf0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol146.txt |
| `sofia_3bodychor462/sol/sol147.txt` | `4304c0d7b9a8058c0f6a5ee26a92126a3d51b56ec11704369ce66176f60b032d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol147.txt |
| `sofia_3bodychor462/sol/sol148.txt` | `7ac84da6f2f6108f219fa31bf1a80774e18ccfd93cb1c91816a8131b78fbc62c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol148.txt |
| `sofia_3bodychor462/sol/sol149.txt` | `0c9e47f86aef9c4fab3d59be42fe7e66a48cd915cf0b4dde02d802f3184eee20` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol149.txt |
| `sofia_3bodychor462/sol/sol150.txt` | `605e215ae281f55f35b359a4f781fa3187cd6e8962269e7b43d2ea2b0be216d1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol150.txt |
| `sofia_3bodychor462/sol/sol151.txt` | `5791ff96802a4e8923b8fd745c618d8761d3d4becf946942475452db4417e2c5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol151.txt |
| `sofia_3bodychor462/sol/sol152.txt` | `e8c9a6cef8646ec9c92d073739a7fd2b6f1cfc806b59fd5b458a8b646a84c25c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol152.txt |
| `sofia_3bodychor462/sol/sol153.txt` | `16fcc980b662c2e81ada3a7c4ee584ee9e4454333a3fbf8e192f82fd3085e006` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol153.txt |
| `sofia_3bodychor462/sol/sol154.txt` | `ec81dd4d58bc6caf71de5fedc496de5896db877ede9bd0e493696d71af03f258` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol154.txt |
| `sofia_3bodychor462/sol/sol155.txt` | `8c0bd73ad71fabe8fb75fa29a908afc31fb6dd7940bcf5e179da3f1c0ee79f35` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol155.txt |
| `sofia_3bodychor462/sol/sol156.txt` | `41cf13096d8e58106ff9c71eedca814a4215ef9b22558dedfdec72be708720a0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol156.txt |
| `sofia_3bodychor462/sol/sol157.txt` | `3aaa97ff0d144f59c2292333eb4e6dd68fb3b3d38c7cb0c18eb0d4e5cd57e624` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol157.txt |
| `sofia_3bodychor462/sol/sol158.txt` | `98e18b5bd354a29a69b57a318740066a72e73477abcbef7816a3d7f88a0c38d2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol158.txt |
| `sofia_3bodychor462/sol/sol159.txt` | `4a4ca23813df401d5d4462f11a6fd44191bd2db0fc13f6d1fa39b7e4e32ae1b4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol159.txt |
| `sofia_3bodychor462/sol/sol160.txt` | `82a2501a2c52b838683d3e0feb7135f24fa0c90b39ddab63b004a29a3b99250c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol160.txt |
| `sofia_3bodychor462/sol/sol161.txt` | `56c767b49e18124284410fb14fe1b4624f2f76cd68d26361334152bc57020d15` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol161.txt |
| `sofia_3bodychor462/sol/sol162.txt` | `6af198b39f3fb5d859279e3899f3a4955e75538adef5bb92b64b54d77490a65c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol162.txt |
| `sofia_3bodychor462/sol/sol163.txt` | `b8642510a1ac102da3fcf08446ac96adec6c1ac9994ffc652973708bc741ae32` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol163.txt |
| `sofia_3bodychor462/sol/sol164.txt` | `e15ac70ee77b0a41cd58600eaca3b20b2c7d924ffe1254cee52e3e457f4a904d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol164.txt |
| `sofia_3bodychor462/sol/sol165.txt` | `1d1ae92bbd2145a396e7f0b5b28fb05343f03872249f06d6bfcaf195a270a41b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol165.txt |
| `sofia_3bodychor462/sol/sol166.txt` | `ff10401f1a69d78e039ad3723be4e7f2fcfb114f2cbb3b94ccbfe87949d93079` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol166.txt |
| `sofia_3bodychor462/sol/sol167.txt` | `52ef3056ae40bd149114e3698f38c8c9864f21e394a48e3686adcb696bac620d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol167.txt |
| `sofia_3bodychor462/sol/sol168.txt` | `c067a56c28ab04d0e71a7dbc644cd9135cfc5c6765fe4b3eb0028a23e36099b4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol168.txt |
| `sofia_3bodychor462/sol/sol169.txt` | `2fdb04dc8f91ad5718ddba99036672b6173a5463c35dd7679b8b40ab57618f1c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol169.txt |
| `sofia_3bodychor462/sol/sol170.txt` | `ac62ac1719047b4f672ef51a25fd174c49f4a4cc293413034c036c2eb1050d98` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol170.txt |
| `sofia_3bodychor462/sol/sol171.txt` | `168843ba8f2f70b6cae80b5642f7395b76b823de4a516fe3b41140d6a7abc0c7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol171.txt |
| `sofia_3bodychor462/sol/sol172.txt` | `aabc08196afef9470670f8e73249241868a954274084158b9ad0c5597188b49d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol172.txt |
| `sofia_3bodychor462/sol/sol173.txt` | `1d276473ab21173a1490964fde20025291a2cb69f37fd90fcb809467d8f5cc84` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol173.txt |
| `sofia_3bodychor462/sol/sol174.txt` | `cb7e3eadbd765b37c0524a26b6a6af84b00e4abff2106ed9e02417fb5c01ab59` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol174.txt |
| `sofia_3bodychor462/sol/sol175.txt` | `f7af41c8d8c466ca2cc38a882b4e6b73c8117164edcf8d0de6418ee7fc882941` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol175.txt |
| `sofia_3bodychor462/sol/sol176.txt` | `86d97a1c73907222353e7e4848af43a98a3f00b5dac2c2a974f9de0d7c343a4b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol176.txt |
| `sofia_3bodychor462/sol/sol177.txt` | `1ad7332d6ad365a8bd7da83d88c3a402bebc82da05abf760396d8a452e6155de` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol177.txt |
| `sofia_3bodychor462/sol/sol178.txt` | `8da1293ee1835fe598457df5939805902213e2a8e7bd4ab35d01e5109181438e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol178.txt |
| `sofia_3bodychor462/sol/sol179.txt` | `b043e47a4157dd4832db572432b8b686983134c11f8d127e6d79b1bc6c455028` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol179.txt |
| `sofia_3bodychor462/sol/sol180.txt` | `4c5f5a5f21c0d7bae074831c496e62b0601c982a9bcf8dcf76c1ad91a7dd48aa` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol180.txt |
| `sofia_3bodychor462/sol/sol181.txt` | `8433fe25f8c6f88e224ae5698868e790dd899f1720af4ee5b01124517c0bb020` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol181.txt |
| `sofia_3bodychor462/sol/sol182.txt` | `9146b1b3d4c4a9c4d75bf08d617c5b8ac0f053f2c19b791c56f45a59e7d283ea` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol182.txt |
| `sofia_3bodychor462/sol/sol183.txt` | `d6b5be4c323f5ebf911a803552509638422600c4264e254c54f47f569a3b430b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol183.txt |
| `sofia_3bodychor462/sol/sol184.txt` | `206fd45b702d02ef19e619c5beb6f56bacf5d3a75d18bd576f86308bca27dc59` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol184.txt |
| `sofia_3bodychor462/sol/sol185.txt` | `9edc47da51f716fc03adc3fa35e38a8dc11c9126dd59095a648d9a27052a82ec` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol185.txt |
| `sofia_3bodychor462/sol/sol186.txt` | `f80929da6d9583465f4ed5de9595e1712ed09a7aa45cf429b78b02d9ebb1e77e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol186.txt |
| `sofia_3bodychor462/sol/sol187.txt` | `de08c4839811152cd73c4d391371c96fb8953ce06176b8db781fd77725208418` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol187.txt |
| `sofia_3bodychor462/sol/sol188.txt` | `313bbad33bd33ffc07e9b6cfe73ca9de370f08caf667454f7d1e46418611fdb1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol188.txt |
| `sofia_3bodychor462/sol/sol189.txt` | `ebe3d07eceab73ff9a26c6a04a28bb8e328e8c620cfe91cc5d912449cb8de4d9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol189.txt |
| `sofia_3bodychor462/sol/sol190.txt` | `1071e871546fb437b8146e7d0e604c6d776d13ea6fb1aaeaff72668fbb42f3dd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol190.txt |
| `sofia_3bodychor462/sol/sol191.txt` | `2d9f739f37106b6b13ffa5fb1d35298012a09585b1f9136ae2d847050fbfb967` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol191.txt |
| `sofia_3bodychor462/sol/sol192.txt` | `43b22fab2fada26eaa5329ebca5f5aeb432e8633d5599524373b156130a4f8ac` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol192.txt |
| `sofia_3bodychor462/sol/sol193.txt` | `614da631d90fc1af8591c161998728a3117719993b588ce8eae32e47481ab6ae` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol193.txt |
| `sofia_3bodychor462/sol/sol194.txt` | `4bbae1ccbcedaae922fccb4f53832c6137f6362b3c5552e4f8f66c2c8904ba4b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol194.txt |
| `sofia_3bodychor462/sol/sol195.txt` | `4dbe2531c79011c87b9bd7c58b85de4ba1eeb30dd019e4278d2a8fc9d2a7743d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol195.txt |
| `sofia_3bodychor462/sol/sol196.txt` | `266cc11f7c44d7008f5bb15f7a974e7f21f979cc9f9475e61d4baf402227a989` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol196.txt |
| `sofia_3bodychor462/sol/sol197.txt` | `9719f98097647591bb64e00151182e75a2906bc9ecc05962e8cf2919b290c5fb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol197.txt |
| `sofia_3bodychor462/sol/sol198.txt` | `1d59a6d05dfede11bb6dfe39701ac3863ff64d9765fec9eb2caf53be7323c53a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol198.txt |
| `sofia_3bodychor462/sol/sol199.txt` | `15f82d7ff7c23ae515ca857a665aea29a47f4d1a19d86df52a2bacea35ccc1cf` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol199.txt |
| `sofia_3bodychor462/sol/sol200.txt` | `79d83a4cef2ff22b9615b2463900c93016db615de52dca5da8fb2f026d54f1eb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol200.txt |
| `sofia_3bodychor462/sol/sol201.txt` | `6cc3c63e78f3aa255efd17feb845d790ccaa8355ece5fd159da6d1f9450b929e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol201.txt |
| `sofia_3bodychor462/sol/sol202.txt` | `b85c5ca436be55a1f6609a08ed6fff690e75be7fd3fa43dbeb631d42ece39183` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol202.txt |
| `sofia_3bodychor462/sol/sol203.txt` | `1ab5effb7938a3fb05c85ec25a3cd0f183aed7dad679078ec16d7f4a007c1151` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol203.txt |
| `sofia_3bodychor462/sol/sol204.txt` | `25e4565fd0ce29a9cb4d4d3272df59f1dcbb5520d8625bed74f7fac948126659` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol204.txt |
| `sofia_3bodychor462/sol/sol205.txt` | `3a17dfc7a44734699778d7e7136fb9e69d813d28ec408d945cc4c5df71c129bb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol205.txt |
| `sofia_3bodychor462/sol/sol206.txt` | `1767c4f0d3007e8d1897bb40cd4f86d6096a41eb0a7b2e53258c0035afa8fff4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol206.txt |
| `sofia_3bodychor462/sol/sol207.txt` | `0be77feae79966f587db4b4b59b95a7afcb0cb90276b7b5245b01619368d3d56` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol207.txt |
| `sofia_3bodychor462/sol/sol208.txt` | `d089586bd38bdb89afccb4bd70bf4d1db66834ce4e42c9a3e8f835df6d1bd826` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol208.txt |
| `sofia_3bodychor462/sol/sol209.txt` | `50acee18f260fa2fcd29e0cdd65f157750849645c4ff5cd684ab8aade48c9514` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol209.txt |
| `sofia_3bodychor462/sol/sol210.txt` | `2620b9ce88f60f2b6e902a3714f7526aa65599bf92ab0dcfc545a85ccc3cad4f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol210.txt |
| `sofia_3bodychor462/sol/sol211.txt` | `44951d4ff0edf8aeb75e1dfe458d55fff47b766ccc5040bac9c61bfce93dbea1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol211.txt |
| `sofia_3bodychor462/sol/sol212.txt` | `ee4edc2df370a4a5b07f9eee55ecb5298d2149b32bcb3c5021038fd8617fc4a1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol212.txt |
| `sofia_3bodychor462/sol/sol213.txt` | `26b4989c7d60dee8068fc518af2772b72d8202a5c26fbd73a7bcd90d825bef81` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol213.txt |
| `sofia_3bodychor462/sol/sol214.txt` | `1354c6b87728fc18d3b1b423297193902dceabf98f3d7643f51ee3881b3574eb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol214.txt |
| `sofia_3bodychor462/sol/sol215.txt` | `967132727b1b7316fa50b181ef15f9c0ee7a71b6a2582f93a64796a842c8758b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol215.txt |
| `sofia_3bodychor462/sol/sol216.txt` | `9a2d4fe4002a471cb55c69ffa4cb186e1c4319d082f3f22ff2305cfa1335c129` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol216.txt |
| `sofia_3bodychor462/sol/sol217.txt` | `3d4c705e75f7005e386733bd79406df9ba1cd8c9a8d4dcab2163d15aead4b87e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol217.txt |
| `sofia_3bodychor462/sol/sol218.txt` | `e5e3db4b6412f30c4bf6fa75fa1c672e239920d660d29537fe2db4b09cd88084` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol218.txt |
| `sofia_3bodychor462/sol/sol219.txt` | `39f01158e4bbb2ba712c069156c690b2f66ee28250300bbde56d1b440abb56d3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol219.txt |
| `sofia_3bodychor462/sol/sol220.txt` | `ce6d433b9bcaba9c77cf9bc110711231e3387efb15ee2c03e1fe204dcf7e47fd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol220.txt |
| `sofia_3bodychor462/sol/sol221.txt` | `cf995b96c6835262cb0210c3de326372ab70b26f110ced8a406d5b86246bad2d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol221.txt |
| `sofia_3bodychor462/sol/sol222.txt` | `6f842e8702717df72088d33aab16bb542ca54d22ddcd27b401db30e25161281e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol222.txt |
| `sofia_3bodychor462/sol/sol223.txt` | `80c32bc9e73e4429920ab229d166e1271c156fafcfc430e1301c2da8170785f1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol223.txt |
| `sofia_3bodychor462/sol/sol224.txt` | `442b7b2b241fa92b5bfb4cd5038c9697cd71fcd4923ad5b8a66b13f9de9d6f39` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol224.txt |
| `sofia_3bodychor462/sol/sol225.txt` | `0abd75c5885f0cc83599780274a3b07d6ad510d194951d72765e1b13752e10b3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol225.txt |
| `sofia_3bodychor462/sol/sol226.txt` | `932d4eb0bc17f73d9bbddafa17ac675ae3718d4a9e7a3aa67579cfd06d9a23fe` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol226.txt |
| `sofia_3bodychor462/sol/sol227.txt` | `88e1fd6805d4d76ac1c803f2e0937e1eff635ef82d7dca64e11d21c8905b9d91` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol227.txt |
| `sofia_3bodychor462/sol/sol228.txt` | `5fb84ff0400eb478dcef132866c2696ef323aaefd643498102b8341db0ed4082` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol228.txt |
| `sofia_3bodychor462/sol/sol229.txt` | `d3e7700eafea4db4331972561d04a100f41b3e5158786a1715ed11dbeade17ac` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol229.txt |
| `sofia_3bodychor462/sol/sol230.txt` | `b076cf2c6fd71d8501e044d03d9774a811a2cf461fd5ffa1d92ec6009448635d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol230.txt |
| `sofia_3bodychor462/sol/sol231.txt` | `7b51c28751d0dcc88a84a6b4984bd01150058ef27bc789dde25e25e0085f1805` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol231.txt |
| `sofia_3bodychor462/sol/sol232.txt` | `02b643d7c63de5186b4afc9ba931cd2d9aba22668e80036605c141da98b39f80` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol232.txt |
| `sofia_3bodychor462/sol/sol233.txt` | `c282cc37e436f364ea9f6ad2b89e9e4c7814cca01289bd2ba8145f66b93ade9e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol233.txt |
| `sofia_3bodychor462/sol/sol234.txt` | `d9a586a8091aa44ae12d4b622234758651c5a6046d7d8d79efe98b36b531956e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol234.txt |
| `sofia_3bodychor462/sol/sol235.txt` | `55087d5a18057d0caed6b226abfca3e9888cdbaf95e0f8e2e54034bf9d4d7662` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol235.txt |
| `sofia_3bodychor462/sol/sol236.txt` | `c07320a37944ba46fc9a7a32f9e62aadde0d7dd81291b213578937cc72df766f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol236.txt |
| `sofia_3bodychor462/sol/sol237.txt` | `3588d9f8c00d6ab58b52304be35b4b6c1976e2f44e01285372a214a016b2d404` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol237.txt |
| `sofia_3bodychor462/sol/sol238.txt` | `f60045066fec5637bcdc50c89e32b06ec0cce819ca74bf89359a548a90387b7b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol238.txt |
| `sofia_3bodychor462/sol/sol239.txt` | `3cef7d446513c2ae97e5c5ded915cb80d5bee5cae7e1dec67a2f5594e93552c7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol239.txt |
| `sofia_3bodychor462/sol/sol240.txt` | `520140b0878799430bed8ebfa9fd041ef54e01b16ff6c375d23784ec35b2db13` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol240.txt |
| `sofia_3bodychor462/sol/sol241.txt` | `c9bb0ec949f260044898b4ce7b7b1b7404032be3f14aeef2161aa06ca27df4e6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol241.txt |
| `sofia_3bodychor462/sol/sol242.txt` | `b9d668cdb3ea2cdb011b8587911427d9aa121c330174e0f55838534406ee2639` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol242.txt |
| `sofia_3bodychor462/sol/sol243.txt` | `a8d915ecb9bd12392a06373d8206c6809894af30352e5f947c84c4337fd18880` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol243.txt |
| `sofia_3bodychor462/sol/sol244.txt` | `838a2eb36e559fed0236909057e88bd155548a3bf25e41a0e941cf6f97bbd54c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol244.txt |
| `sofia_3bodychor462/sol/sol245.txt` | `1e595470f0d9900383cc032c30d29ec8545a65f93e89ae3a14c72dbed22f6574` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol245.txt |
| `sofia_3bodychor462/sol/sol246.txt` | `4e16524aa8ad94e730ad7db04e985f1ff442dc42e3764f669ea2913bd30254e2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol246.txt |
| `sofia_3bodychor462/sol/sol247.txt` | `df7b8575e3eee89c03ae108caa294408c511ff3d304bf71abdd306a0421fa1fc` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol247.txt |
| `sofia_3bodychor462/sol/sol248.txt` | `89b054068ec9e41b4f572a84b2aff27077124b3ed5c548dddbadb699e1e693e5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol248.txt |
| `sofia_3bodychor462/sol/sol249.txt` | `238e96d62d90915fc471edd25b8d6dd1e370f796c949b5a946d091353161a514` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol249.txt |
| `sofia_3bodychor462/sol/sol250.txt` | `0d895b5621e21768c4df61a8231a588d5170dabc290941254049daa6d6d476cb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol250.txt |
| `sofia_3bodychor462/sol/sol251.txt` | `20039f691ad33e6c1dd6bffcdb753020f815277ab48ce0b4a66c24dcccc25c8a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol251.txt |
| `sofia_3bodychor462/sol/sol252.txt` | `56978f365204d4896a5a8ad50170f699414c67cac69855204720d7924a80fa72` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol252.txt |
| `sofia_3bodychor462/sol/sol253.txt` | `64894a8ef50b98b46dfcb368f98d7d44934ccabd6f26ea0d188d4d73df5ae20d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol253.txt |
| `sofia_3bodychor462/sol/sol254.txt` | `12e5ab0a8c3fabf5b9cefd07f9d0006e038facfbb6f6fc2ff1293a550cbd3901` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol254.txt |
| `sofia_3bodychor462/sol/sol255.txt` | `0ff7b4d97cf9acc4f99f010b08422be3544a470d2ca8a3d8b018557e8f155025` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol255.txt |
| `sofia_3bodychor462/sol/sol256.txt` | `7f3bffaa4a2f380500601afe387b798c06ed04337dfe36cff3f537b742a25a57` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol256.txt |
| `sofia_3bodychor462/sol/sol257.txt` | `8d12fd3c365d9819875b35c677e2bf92b44dbfb4e364092f3b3d3015ff0a38a7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol257.txt |
| `sofia_3bodychor462/sol/sol258.txt` | `0b249ef259f30f09b8dd9644e794e058e914de84967731696cf82c348dbffd85` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol258.txt |
| `sofia_3bodychor462/sol/sol259.txt` | `835c69bc6b83fc6bac8b23fd1a45e266971380ae7659075229570fc83398c0c6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol259.txt |
| `sofia_3bodychor462/sol/sol260.txt` | `e842f5122a00f3abf6b0d25f0c3a98577ca5c3e34e6313966df9909eaede66c5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol260.txt |
| `sofia_3bodychor462/sol/sol261.txt` | `eb26db2b9438cc15d4149bc451d0210ea43fcc9fcec89d1d8a1a80fe029b772e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol261.txt |
| `sofia_3bodychor462/sol/sol262.txt` | `5d00eccdfbfb8480fd86e0b06e4108b181c717acf0c39188a78ca066bce677b9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol262.txt |
| `sofia_3bodychor462/sol/sol263.txt` | `11f7771d7670c121e1b794ea01c090ec3ebb8519689e4be5d2aae4c0f53a1721` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol263.txt |
| `sofia_3bodychor462/sol/sol264.txt` | `f1ffd94116ea7572fd8fd163aef0ca458adc7d88b47507f086e96a4fa9caf437` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol264.txt |
| `sofia_3bodychor462/sol/sol265.txt` | `b22b1463eebfa11ffca1028973675d234956d38372a60e297b1fa74dc24b0cb1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol265.txt |
| `sofia_3bodychor462/sol/sol266.txt` | `7e373d0823b929bb54c35095ce86ee89698e64e354d7742e9f90fe7b814ea01c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol266.txt |
| `sofia_3bodychor462/sol/sol267.txt` | `74b4697458b5c31a42a762209c1d09485e949f7a5ce03371cb9ace5f093d1df6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol267.txt |
| `sofia_3bodychor462/sol/sol268.txt` | `a311fa0862da0d135afafa1321211a31366282f80a05cd0dd07bf3965b4df0c0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol268.txt |
| `sofia_3bodychor462/sol/sol269.txt` | `8b7d561f7d2219b6caf40125ff4893ba162880af076aa1fe68881a7183144fc5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol269.txt |
| `sofia_3bodychor462/sol/sol270.txt` | `ee75f3a868be6e01b9b1a20f80ccb43d974bfbf10d89cb7b63cde7c2db94d9ca` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol270.txt |
| `sofia_3bodychor462/sol/sol271.txt` | `aa0e61dbc3cab98f8a6d4a58911f32f18fb6255b167d0fa05ee04f6afebc4bca` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol271.txt |
| `sofia_3bodychor462/sol/sol272.txt` | `5439e2017edf23c4b1746d090a433a03c6832e89fcb801850de8c641b59b1b4e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol272.txt |
| `sofia_3bodychor462/sol/sol273.txt` | `23faf11b7db9e7c328e9ed032219ee7f671099928f349386ed763791978af059` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol273.txt |
| `sofia_3bodychor462/sol/sol274.txt` | `f41c54f8042083a6db57abdba2207d9af319d9508a1a91bd058663a3c4e8b5fa` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol274.txt |
| `sofia_3bodychor462/sol/sol275.txt` | `f9a6bb0bcf7f384c86513da48549ddc4854d8459d3ba123c6b72abccce5f97e9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol275.txt |
| `sofia_3bodychor462/sol/sol276.txt` | `8cc2d93d145aa5136ffe9bcd50fd24edb9a0460f2ac167bbc917ba95bddd4aa7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol276.txt |
| `sofia_3bodychor462/sol/sol277.txt` | `a17b3d71699a0ac51f5d8f85c8deef8234d4cea3a93ab3df068a4fed5a5910bd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol277.txt |
| `sofia_3bodychor462/sol/sol278.txt` | `aeff91f5d82a8eaf860897cf2bc3bcad448ff3b60ad141824995b360d7edbeda` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol278.txt |
| `sofia_3bodychor462/sol/sol279.txt` | `e0e274e8c2f98e6c846539ac1c6f3709dc136bb04977f35b9959ded31c63c7f9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol279.txt |
| `sofia_3bodychor462/sol/sol280.txt` | `20c05bbb1272d760c2d6d4657e7f72187a0857f8405b37e092af8c8e4028e902` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol280.txt |
| `sofia_3bodychor462/sol/sol281.txt` | `8c05a9318caa620c1418af09318576a02ed6601aaf1586e1978532fdb17e3178` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol281.txt |
| `sofia_3bodychor462/sol/sol282.txt` | `9b17d1a64b09eb41d43956d1b78c80b710fef82d67cabc57026fbe3e54a8cefe` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol282.txt |
| `sofia_3bodychor462/sol/sol283.txt` | `c36669c56767325d7189ea052ac23b31d6406a3e59fe7c5cc3b75883b54677e0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol283.txt |
| `sofia_3bodychor462/sol/sol284.txt` | `c637f6dcf321c9eab9f9be1a91897463fdc5f6004a298480ff621d650f9ed99a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol284.txt |
| `sofia_3bodychor462/sol/sol285.txt` | `c4edf4f2a0b27fdf73d7997299b13f89949f503a4c7339df7e7c0d3d83504b22` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol285.txt |
| `sofia_3bodychor462/sol/sol286.txt` | `6b44282a1390ab9aac1dc4add1442e11925f1d45daa1840b4f1155ddacaafdc9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol286.txt |
| `sofia_3bodychor462/sol/sol287.txt` | `bd62fcce8bfa1d6f47bae5762d82eb6a982f1e9a64e3b6d172dabc233899f036` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol287.txt |
| `sofia_3bodychor462/sol/sol288.txt` | `0a1ba6bfd573f8a7d17a59bfd2f003c4fd824393eb17e6b143745ee11c1431e8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol288.txt |
| `sofia_3bodychor462/sol/sol289.txt` | `006e546dd8e298c267e9bccbe3f0b2a209a0abb0364a87bd390fa5f34216c49e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol289.txt |
| `sofia_3bodychor462/sol/sol290.txt` | `31882081996ee160ed2ec150fc6146afd81301e859c7c1bf5efbee6904062d72` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol290.txt |
| `sofia_3bodychor462/sol/sol291.txt` | `70d885902d9fff7e0bf31b7ee9020493cbae67ec099e1351eaad931fd465fc6a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol291.txt |
| `sofia_3bodychor462/sol/sol292.txt` | `30d315a217c45834f07db8d8cd1a6826dbccc614662d54d3fc97cf0f02951767` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol292.txt |
| `sofia_3bodychor462/sol/sol293.txt` | `ec4f8cbaa7c9189fdd7fe2c35f3d26ee295655c8f1943749de3e0da24f71b905` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol293.txt |
| `sofia_3bodychor462/sol/sol294.txt` | `e7bd12d275e4b44a412afe8cba5f6eeed814e80dbab6dd5d833245f97bdeb3f7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol294.txt |
| `sofia_3bodychor462/sol/sol295.txt` | `250a668b1ac5c1a095646c42f555adc75f3b5c36ba670637d527e5d5e2a78be6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol295.txt |
| `sofia_3bodychor462/sol/sol296.txt` | `fa94973ac58909bb7a635f7122f8b9b2298862a07954e22997b4493fc89390eb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol296.txt |
| `sofia_3bodychor462/sol/sol297.txt` | `85eb3a1f9d3a39f573e8ddad6d99f2cd2862bfd84abea6aacd0fca7e3664f8b3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol297.txt |
| `sofia_3bodychor462/sol/sol298.txt` | `bcf7eec4eb4886c608f58a7071e15d1294e0f14273f416a0444b84591ed4330f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol298.txt |
| `sofia_3bodychor462/sol/sol299.txt` | `b9d8505741abd03921e1ee23a1e2eaeff285b3c87982b22e0dbac0e178e3f732` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol299.txt |
| `sofia_3bodychor462/sol/sol300.txt` | `d831802b5f97479d21dd9bc60b299fb7131f5b28a2af6cc3ee2e346d097d7301` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol300.txt |
| `sofia_3bodychor462/sol/sol301.txt` | `116e750284c5a09da5d7e837908df4ee237b78b1b1bcca2ab77b1dceecb986cd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol301.txt |
| `sofia_3bodychor462/sol/sol302.txt` | `1c4a823bf13fdcd8b138cfa27e6e242bbb53fda5cc90c44864ddd78120490626` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol302.txt |
| `sofia_3bodychor462/sol/sol303.txt` | `6310e55967c786f6d57876ba633365fc0c75d2e95a829567b99f6ea64c763a2f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol303.txt |
| `sofia_3bodychor462/sol/sol304.txt` | `d2ea107ed77f3f5270a5cb66db643b617e82e85bc9f9b7ca562445a1d7c84748` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol304.txt |
| `sofia_3bodychor462/sol/sol305.txt` | `4b796049227b43433b19d62dc58b341d1aa82b60f70b5843becab90652fd2dc6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol305.txt |
| `sofia_3bodychor462/sol/sol306.txt` | `589571941af50360efb391c75bf1028aeb52177554e1c5d9a105ba9cc8aaf975` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol306.txt |
| `sofia_3bodychor462/sol/sol307.txt` | `a69714f428f7df74ffd94b3fa3b7396cac35a774265f349720c23819eb81d5b2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol307.txt |
| `sofia_3bodychor462/sol/sol308.txt` | `394822c2135370a5a4d573b963e2e43cb57b05dbadf306e4448fba6a91d3cf92` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol308.txt |
| `sofia_3bodychor462/sol/sol309.txt` | `3b8195b15087d81b4b480db9a3c6181528b4620311bf4a60cbff890fbe6cb4d3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol309.txt |
| `sofia_3bodychor462/sol/sol310.txt` | `665ed042f1665ba71cab547c03d180fc24695feff6ecaed90b1e83bc8422ad60` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol310.txt |
| `sofia_3bodychor462/sol/sol311.txt` | `88d7c1413665cbbb647952149d1ea0045a0b07518fabdf3e4944f5999f96df84` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol311.txt |
| `sofia_3bodychor462/sol/sol312.txt` | `cba96da8b23086ce625a524b916636762f2801cd2067a1d54a7c2618c91c4f17` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol312.txt |
| `sofia_3bodychor462/sol/sol313.txt` | `20ef3f16ed5fe41cc73ca5c76fd6a49b2bbde9527ec525c77185431f4e301e5a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol313.txt |
| `sofia_3bodychor462/sol/sol314.txt` | `45c69a8c1f7ed64a83424bb50a9c0ba357f842fc5510051c38064fe84fde7b35` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol314.txt |
| `sofia_3bodychor462/sol/sol315.txt` | `0b6a9d87d7a103a0768b9abb98afa2087dd557b95a71f03c33a3d5dd397474cd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol315.txt |
| `sofia_3bodychor462/sol/sol316.txt` | `dd352dea22ea9cfa9832e6bd833f7fa9a1bafc793ee023ec186a3c747692c367` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol316.txt |
| `sofia_3bodychor462/sol/sol317.txt` | `f40ac64153620b23c0050a0dbf34f8b69477ac455e372228c585633b5b7ecc62` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol317.txt |
| `sofia_3bodychor462/sol/sol318.txt` | `79d25fb0fe0cc05fdea232944a903ad45d9f0d1cb96f2b0d3cc177e25028bad4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol318.txt |
| `sofia_3bodychor462/sol/sol319.txt` | `9171e61d4b96b2d01cb3a01ae3143b284758fee6641b62d3570faa873f0c5546` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol319.txt |
| `sofia_3bodychor462/sol/sol320.txt` | `2222e8543c82c26ed60ff4c221058c65ced974bf513065cf0c9cc31b163596d4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol320.txt |
| `sofia_3bodychor462/sol/sol321.txt` | `43a9862b3b4ed2ecce97c526bcb7060235c38926a8f96866df299c76ac6b8645` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol321.txt |
| `sofia_3bodychor462/sol/sol322.txt` | `65e1721fc2262f13d424cbbc6dd59a4067f3ce8902bb8724007cc9b26218ba79` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol322.txt |
| `sofia_3bodychor462/sol/sol323.txt` | `5940a387516e98cf3616a0d863f00a7f5230959c4082954e44c329aa48c7ec8a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol323.txt |
| `sofia_3bodychor462/sol/sol324.txt` | `e9039812dd759edb9a9072779a3a3e19ca27f7de77b46ab3688af1e1b0a0adb9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol324.txt |
| `sofia_3bodychor462/sol/sol325.txt` | `ef32fca04cf72f745789ad6fffe3e3159ffcf8801512e28c244cfc9fa6d20d7d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol325.txt |
| `sofia_3bodychor462/sol/sol326.txt` | `d9892a0bd0706240bf8d48c9d1d2c9125fbac0ec641083112392198900900502` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol326.txt |
| `sofia_3bodychor462/sol/sol327.txt` | `e448299e949ea4d88e1af860721918f4c59d25333e596e24fe4bafbbbc96438f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol327.txt |
| `sofia_3bodychor462/sol/sol328.txt` | `eb7b53968dcb91266f0cc4f1a6dfc7517b86917987485f24ad7d86cf9c8db6d4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol328.txt |
| `sofia_3bodychor462/sol/sol329.txt` | `f838052646702971c5147e2f6941ac9021b09d289487bae5f9608e8569eb961a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol329.txt |
| `sofia_3bodychor462/sol/sol330.txt` | `e9794ef9e5354feab5020283066fc1468207af1d39489416745e0bb92841e993` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol330.txt |
| `sofia_3bodychor462/sol/sol331.txt` | `1052bfe6907d43707a0e082b34096850c5e21dd690e8248ede036059255173bf` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol331.txt |
| `sofia_3bodychor462/sol/sol332.txt` | `db40674738f41424460e6f91e8608bc657f5b16e171338d9bf309d4b89c86dfd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol332.txt |
| `sofia_3bodychor462/sol/sol333.txt` | `3c95f0f4a3e94da5a796e2bc2e361f6b0ed6d75c44e14b276715c7d32ad6caf2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol333.txt |
| `sofia_3bodychor462/sol/sol334.txt` | `209083a5673758c19dcf25ff899121461c3ee1ed18bccfeb8c703d3333dae1e8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol334.txt |
| `sofia_3bodychor462/sol/sol335.txt` | `2d121aabf0bdd1a9191da546c9cbdfd2ece7160babbf8db8be65622c6e63760a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol335.txt |
| `sofia_3bodychor462/sol/sol336.txt` | `c3da477cebbe2f94709ffbb1233b672a3c8b42994230e4fdc0737358d9e63bd7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol336.txt |
| `sofia_3bodychor462/sol/sol337.txt` | `44945937770f9778ba4444aa9974f5cc5543ba56646aa0542e5a363a6e59e38e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol337.txt |
| `sofia_3bodychor462/sol/sol338.txt` | `ebe6dd6def8cd3ff5edc39173293b0932c1e0ac8735172c82864c03e97641577` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol338.txt |
| `sofia_3bodychor462/sol/sol339.txt` | `7daebddc10f05f6a04151a5754055c3ab5934ec669536b00a05821952eabd8ec` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol339.txt |
| `sofia_3bodychor462/sol/sol340.txt` | `06a827e41b08a17c1d022d345f2f1278dd5cddf7d213bf60c7a3eaefcb962051` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol340.txt |
| `sofia_3bodychor462/sol/sol341.txt` | `de9a3ce0e3d70efcf6c7abbbc6945af9a33eb59dab1ce890229c96654d479ca2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol341.txt |
| `sofia_3bodychor462/sol/sol342.txt` | `d7fa8509e95f111b52db3b41db51745ca116e7f1cecf48a121370f64429c3710` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol342.txt |
| `sofia_3bodychor462/sol/sol343.txt` | `b88936d8db44b2a448e63487f845e9bcf66cde8d9793e25880444f7f6d167873` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol343.txt |
| `sofia_3bodychor462/sol/sol344.txt` | `93fd8b8d69d0a7ed6fbc6ce3b6bc6542359158c8a2a87a3825f100c5e5ef2f06` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol344.txt |
| `sofia_3bodychor462/sol/sol345.txt` | `4b564b9e69b619979de1182fabbb657d73e11c5dfd8f10088b811a8cec1d0c25` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol345.txt |
| `sofia_3bodychor462/sol/sol346.txt` | `7a8a35ff45ca5ee9f2bba23958399a78a75c371118d4730d6b9455ab46184134` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol346.txt |
| `sofia_3bodychor462/sol/sol347.txt` | `e43fdc781eb3aafa66797702266c278d14fdb2c305238bfa8b1dd09f8489594f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol347.txt |
| `sofia_3bodychor462/sol/sol348.txt` | `2ea67d63ea615eccf135b79d7d63935ad195552c17af294b1d3e661093c6601e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol348.txt |
| `sofia_3bodychor462/sol/sol349.txt` | `b866591db683a9da5298fce66f6cc801c8584529c93336e50dd1cbd2159f8ded` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol349.txt |
| `sofia_3bodychor462/sol/sol350.txt` | `9aada385ece8ac45ec38d1a909e86420b6a775f8d79cdba2c7145d1ae295f82a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol350.txt |
| `sofia_3bodychor462/sol/sol351.txt` | `e39344dfb9f6750941c5140c761d40568f0152c86cd09c53fb71f75142953892` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol351.txt |
| `sofia_3bodychor462/sol/sol352.txt` | `42452bba3572e3ccb09c887176edddb3094bc589ab8d6ac757175089e658c42f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol352.txt |
| `sofia_3bodychor462/sol/sol353.txt` | `d595b38e7bb08cfefef6b26e40e0a26196621e100814fc1da1f389309bd2b68c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol353.txt |
| `sofia_3bodychor462/sol/sol354.txt` | `a26313d1e839c0b23dabd3d928d3aecada1af38c5eca9990e062305168559c5f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol354.txt |
| `sofia_3bodychor462/sol/sol355.txt` | `0902f7985ae3641b204df47f8c1c2925b8ba7d5381db3863e2162331353491e9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol355.txt |
| `sofia_3bodychor462/sol/sol356.txt` | `11b5a4750d1fae3c75145a6f0cd266b51e162a348427e9c90e257089d5ad9299` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol356.txt |
| `sofia_3bodychor462/sol/sol357.txt` | `52d80c6683db32da7499849e2a4364d88b9a7cd6c893c0f626b7a12f2eea2e46` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol357.txt |
| `sofia_3bodychor462/sol/sol358.txt` | `0b8ae21612baa71c1844f20a8d8fc2a82bbb482870fb429a646241c3bbbddd7a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol358.txt |
| `sofia_3bodychor462/sol/sol359.txt` | `e91e72521127c8d0d9d4a76c207fead863e839765629238af06035276b818679` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol359.txt |
| `sofia_3bodychor462/sol/sol360.txt` | `39e0b854ac3307fbd13b011cc97de45d5b3c60c67a47c24d4c8566798c843e54` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol360.txt |
| `sofia_3bodychor462/sol/sol361.txt` | `9be7653dd22a136d325ca4f083939159f11a8853d5233cd22a4346a83337a78b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol361.txt |
| `sofia_3bodychor462/sol/sol362.txt` | `47cce8a6ec0e9c4ae858ba11ab04d3484b93b9def5c8d0d05b023d225d210388` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol362.txt |
| `sofia_3bodychor462/sol/sol363.txt` | `d8bd153fb24cff99a84709d755f5cacf4e07fee3c3bf8ec8152d117bdf6e747f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol363.txt |
| `sofia_3bodychor462/sol/sol364.txt` | `af7155dda5a19dcc1805e90c629af9389028ccc98e415b1a72c2ea804c9853c4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol364.txt |
| `sofia_3bodychor462/sol/sol365.txt` | `f2c547794f2c768a0f4744258514c6424aa93102d7d2354f355cdecdaae97e39` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol365.txt |
| `sofia_3bodychor462/sol/sol366.txt` | `cdd471d76e73aefb29099d555ae25b5ef4272df78cd87dfa9fb467f3278d2f68` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol366.txt |
| `sofia_3bodychor462/sol/sol367.txt` | `02ae2fd2cb54eed4c4bab59255207a8b5c3f290e5497785a2f48ec9e439afb30` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol367.txt |
| `sofia_3bodychor462/sol/sol368.txt` | `fbc887e67b4d1676efaac6740984d5bf51cb434301995872d017ba65e1978dea` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol368.txt |
| `sofia_3bodychor462/sol/sol369.txt` | `3dd1affe3723e4cb012dce816fa47da154edd18530398f385db146423f1c4534` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol369.txt |
| `sofia_3bodychor462/sol/sol370.txt` | `6e5f9e7b1bb3eb745800ac1c9cd3bf4e11a3e26fbc55a60fa423db7cbbdbcab2` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol370.txt |
| `sofia_3bodychor462/sol/sol371.txt` | `b4ae79d61d35fa3f6b9b98447bdb7dd826cded9cba33d0e874fd2d8da5b92cc5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol371.txt |
| `sofia_3bodychor462/sol/sol372.txt` | `a6fae1d66c952fa264cca9dc5a1ad2c6ea41dbb460a845dd0db489fbfdb6c0d5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol372.txt |
| `sofia_3bodychor462/sol/sol373.txt` | `67355e09be7fcc253661b270d673646301ec780a92316a9ed12e3f9b02241600` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol373.txt |
| `sofia_3bodychor462/sol/sol374.txt` | `d6690090bc4f812d9511583fbf8fe09d2eedfd8dc2d42c7608712ce89a22cf59` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol374.txt |
| `sofia_3bodychor462/sol/sol375.txt` | `b444d82ed34bd6cae12bb8e82260da4f238ae080171d09d153289041d278de84` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol375.txt |
| `sofia_3bodychor462/sol/sol376.txt` | `36dd5fe6b08b70f89a1433200cb04bf9d1a3b1ed222643f424581d71233000ec` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol376.txt |
| `sofia_3bodychor462/sol/sol377.txt` | `443f62ca2b2aabfc130c23bc0d33d59add2dfc33a068ee6cbeaca79aca4b70db` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol377.txt |
| `sofia_3bodychor462/sol/sol378.txt` | `e67390245ab5497669428232a160e902ae0c91b21caf9978bbefce964f5eaeaa` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol378.txt |
| `sofia_3bodychor462/sol/sol379.txt` | `ff9ea02960b336d0f4725844b331d7c22d51aba9a914306e74cbaa877b00d231` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol379.txt |
| `sofia_3bodychor462/sol/sol380.txt` | `af6dbb7c25287751b0ddea446ba48ceab4ed958bd40530237f76f4b16ca4a347` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol380.txt |
| `sofia_3bodychor462/sol/sol381.txt` | `86a7c508ebf2ab195c0f9e30277e1348b888582dbd88548216fbe30f7577410e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol381.txt |
| `sofia_3bodychor462/sol/sol382.txt` | `bb57a9e9a338986d4d7d4264da640e65c182f56c2981dfe6381df19a38c9285a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol382.txt |
| `sofia_3bodychor462/sol/sol383.txt` | `e29d9590b4962d8dcf2e9d28073b6b461331770e625ce03fbd1824c1c712191c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol383.txt |
| `sofia_3bodychor462/sol/sol384.txt` | `ae24467806fbba22b84e35f937b28056a1d53ef770b59bab50faa2df003b154e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol384.txt |
| `sofia_3bodychor462/sol/sol385.txt` | `5842452261500cab05fe970cc47627bb0e918d5167f9819af5ab41014d2d6fe9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol385.txt |
| `sofia_3bodychor462/sol/sol386.txt` | `b288e068fbb955fb5060f2cbc835eb4707b4207d6cf42773e260ea846f5b2416` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol386.txt |
| `sofia_3bodychor462/sol/sol387.txt` | `c2741aff0683978f4721e440b7386e5a25480e6d9b30571a9863cf2d30f6021c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol387.txt |
| `sofia_3bodychor462/sol/sol388.txt` | `b55ff3c9ae2844fae090e683efce868dda0aff9bc0d8b5bb51bd621caee124c8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol388.txt |
| `sofia_3bodychor462/sol/sol389.txt` | `cbac511fcb91b77110265aed62c747fd9dad8216e9a09950dd23d56f1ef135bd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol389.txt |
| `sofia_3bodychor462/sol/sol390.txt` | `232289b49aa1bf0a586410d1ee7042e189352251e92a94da2540da5d4a77bee4` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol390.txt |
| `sofia_3bodychor462/sol/sol391.txt` | `9a7e10dfbc97fcbacac19e35db34755c783897c0ec2c5654299d3284ea14fb23` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol391.txt |
| `sofia_3bodychor462/sol/sol392.txt` | `21168594d22bae359cc6f0d4af660ab2603a38bf6e8309af22c4e38c0ea409e0` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol392.txt |
| `sofia_3bodychor462/sol/sol393.txt` | `50427fbe578f1899d40dd21115f1bb1cbcf72604131d5221ca0863ab9a3d8f3a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol393.txt |
| `sofia_3bodychor462/sol/sol394.txt` | `ccc4c9a422436e95b77523b7eb3cd79efebea46d16ae9cf8ac0f8112fa483505` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol394.txt |
| `sofia_3bodychor462/sol/sol395.txt` | `ed5bc630d7d24f7da7aaebe64a49252ce8a03c7d707a59c8691cc3ec1af4ee51` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol395.txt |
| `sofia_3bodychor462/sol/sol396.txt` | `a05b5457c1c0212cd21be2295c2681c7b0aa4039ecb0e56f7022e9a776bc85cf` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol396.txt |
| `sofia_3bodychor462/sol/sol397.txt` | `8b1df6b32551dac40129c3fbd4b2b6cddec824de93e7e35eec0d0e0c0f4fbec8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol397.txt |
| `sofia_3bodychor462/sol/sol398.txt` | `2cae6f8b28fc80eed2aa4cc74e5243c74139848e3e443b18c619dcac505d9801` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol398.txt |
| `sofia_3bodychor462/sol/sol399.txt` | `1b58e959dd1145830a60a3d355df56744afd749be36abf734e47d121ee249bcf` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol399.txt |
| `sofia_3bodychor462/sol/sol400.txt` | `de656dccea0cdda345823709b1939f35a5f5f57a551ca92bf3c0bb084ce2fb9e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol400.txt |
| `sofia_3bodychor462/sol/sol401.txt` | `799b01a883a86d6efb191313e6cb793f83a0c20f438381319955ca5dacf501ca` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol401.txt |
| `sofia_3bodychor462/sol/sol402.txt` | `fb6403d42315056fabd9a270da491a048045913aa10835207d43dc490313030b` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol402.txt |
| `sofia_3bodychor462/sol/sol403.txt` | `b0588d245a4a5ea1eb9c9a73b2c07a9b1003ceb51db76ec0892bd37dca8fecdd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol403.txt |
| `sofia_3bodychor462/sol/sol404.txt` | `bc907eaa36875dc9cd72b6cc4083cf3c1077159f260fc9c1ff688928f7a29c1c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol404.txt |
| `sofia_3bodychor462/sol/sol405.txt` | `f95429247d4ee4888352ec1848154f055b915eedd1c99b91aa5a92eefba819a6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol405.txt |
| `sofia_3bodychor462/sol/sol406.txt` | `f1bfee1401a674fc137c2777795b57d74e018d4982debcb1a02a35da5e4ffa1a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol406.txt |
| `sofia_3bodychor462/sol/sol407.txt` | `ee32d4dd5159b02f5830db5ca274e2ba35904ba8ef706397842f157c22bf6417` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol407.txt |
| `sofia_3bodychor462/sol/sol408.txt` | `e7b6859ddf53fdf17a19d1ab929eabc455b3cc043098d8b5644250e4f1fa334c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol408.txt |
| `sofia_3bodychor462/sol/sol409.txt` | `f865ab1e6ac812f0910fd4608dfdc4b203916d6d1bb4e516fdad380ad9728b14` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol409.txt |
| `sofia_3bodychor462/sol/sol410.txt` | `a79cdadf3887c2ee23c6e52e07f2da4f10e6a6ce82a8cf5389bbb644cc107592` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol410.txt |
| `sofia_3bodychor462/sol/sol411.txt` | `f5eba20eb6bad01d5cc1194d3c87a797a4e40882e807af02d19ac69ebaafa448` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol411.txt |
| `sofia_3bodychor462/sol/sol412.txt` | `1628423977b6128768c6c7afcd3d7d07857db932259305d474ad172cf4ec64ec` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol412.txt |
| `sofia_3bodychor462/sol/sol413.txt` | `4d92f484080735a245baca3d5847fbf36aa9f52a0fc521b4e3112ec6b56dcafb` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol413.txt |
| `sofia_3bodychor462/sol/sol414.txt` | `9690c73fc1113ffdcf2bf5c1c1dbcefea2a2c2bbe46dbd3399fb76b76418da89` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol414.txt |
| `sofia_3bodychor462/sol/sol415.txt` | `8703a3afaec434ba5e070b193b3090fb5b5c94dc939012cc6b4abf621e470a4d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol415.txt |
| `sofia_3bodychor462/sol/sol416.txt` | `47de3c05909d23be77247aa11b18ceae4d1f591765ae24a83f35fc917afd13c3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol416.txt |
| `sofia_3bodychor462/sol/sol417.txt` | `34eededa4730324328d12ee07fd0f88702f1ae365fd811b60a0501b20f5375f3` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol417.txt |
| `sofia_3bodychor462/sol/sol418.txt` | `0974c5094c216e98362ac9f81a9727465c505a93ecc42498cbdfbcf2589485ba` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol418.txt |
| `sofia_3bodychor462/sol/sol419.txt` | `bba852f564b097717ba5abb73ae64d9987d6f6a09b9795cf128bb4d5bf429032` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol419.txt |
| `sofia_3bodychor462/sol/sol420.txt` | `03f2aa3f93c490cb3ea8fae8d9f0b99a57ce7ae57cc920892d60c90637fcac94` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol420.txt |
| `sofia_3bodychor462/sol/sol421.txt` | `5db526b8995617ac4a7935ce3561d0611469233763de9e034d1b069b782b0b9c` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol421.txt |
| `sofia_3bodychor462/sol/sol422.txt` | `6e86e63d19fcda423a8694bcc8ae8bac4a28638834107acc8e99a8c16dea17e9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol422.txt |
| `sofia_3bodychor462/sol/sol423.txt` | `1aeaf4c06efb7fc53d16b6a5cf6be4bd0d4db6b2ebf54fb973e5e645ccce4678` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol423.txt |
| `sofia_3bodychor462/sol/sol424.txt` | `2e183f90e919ee962fe1fcca7513db703c143911b1e33787df5839a486ab2726` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol424.txt |
| `sofia_3bodychor462/sol/sol425.txt` | `09b6a891cab2428ca11e0e7ed3e54ee1803db26e1805b6c207713871ae8a1331` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol425.txt |
| `sofia_3bodychor462/sol/sol426.txt` | `7752404a36c958d86e4dbdea736ed34e3de122c6bd5194ad1151474b88ba71a5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol426.txt |
| `sofia_3bodychor462/sol/sol427.txt` | `558f30d0c9ede612ad213a640fc4eee300dcd68a8c6d18c60199d2353c40f94a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol427.txt |
| `sofia_3bodychor462/sol/sol428.txt` | `cdd92894e2772a4119277512ad3a203b3a08db340ab1a165453a21c43f51eb63` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol428.txt |
| `sofia_3bodychor462/sol/sol429.txt` | `f724b99db5e9f5e6ea4d797a3ff1a1993417e827099de103ca7f41c9df571a72` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol429.txt |
| `sofia_3bodychor462/sol/sol430.txt` | `1d04aacb1dfbc625fe2bad451bb5f49820f513a7a1cfd88919fd403507617452` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol430.txt |
| `sofia_3bodychor462/sol/sol431.txt` | `2c55a80bff78d8599ddd3fc68ee34fe749afad85535c07e4325192b243527e84` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol431.txt |
| `sofia_3bodychor462/sol/sol432.txt` | `5b3eb886f44d8f0d1de5e3b94a1fb6e4cc2a89393fcf4501a79a2d6f5c19aaa7` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol432.txt |
| `sofia_3bodychor462/sol/sol433.txt` | `79dc8e16d68839a82bb40af3f6607d5c9061924229d4907dc21988bfc5d8cca8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol433.txt |
| `sofia_3bodychor462/sol/sol434.txt` | `d12e61f6c8de79e18e3bc071e65264d78ad1a3b6763ce0c4a29b1e99911c8658` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol434.txt |
| `sofia_3bodychor462/sol/sol435.txt` | `21fef9704779fc7eed595d4c2f6b574361423f56043a96769834ce97e0219f56` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol435.txt |
| `sofia_3bodychor462/sol/sol436.txt` | `663a4f7391a0d7a7d06b9737fb21ae9d6a9682a0f92a60e32bcd9a6b21b82de6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol436.txt |
| `sofia_3bodychor462/sol/sol437.txt` | `2f5dd328c826a20d42bfde2387f3842dcc8ecdcfd8c943a891f4e6fdb0adb229` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol437.txt |
| `sofia_3bodychor462/sol/sol438.txt` | `9a6800834eeb23ab4eb2afa43330d0b347ee955877c2161fae3b27e866cdd771` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol438.txt |
| `sofia_3bodychor462/sol/sol439.txt` | `e573e80c633b517ef75d9635cfae9c7ea3e6b89f654c4c6ddaeb6135b16bfa50` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol439.txt |
| `sofia_3bodychor462/sol/sol440.txt` | `340d15cb831ac7e6614e875446cd56171db66b4a39e233adc7663af6886a674d` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol440.txt |
| `sofia_3bodychor462/sol/sol441.txt` | `11926a3774c2612110182947973318bf17c4d505e22cd355e8746ad775766fc5` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol441.txt |
| `sofia_3bodychor462/sol/sol442.txt` | `7d76ca1827fe02074b9d25ff76895251a1853a05f7d79d2ed5219dbb66188bf1` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol442.txt |
| `sofia_3bodychor462/sol/sol443.txt` | `ff770e0e1cadbe8db7d51459d21a032847146fd104f03300f990148a6c044235` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol443.txt |
| `sofia_3bodychor462/sol/sol444.txt` | `f2115f0f9f7c88ac9a268fd3a22f1ebd973eb292f5aaa64b7ecc0038d2e10c19` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol444.txt |
| `sofia_3bodychor462/sol/sol445.txt` | `5a2e272b69f06d62b74910043da329f12259f0150b22545f1fb26d12315fbd31` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol445.txt |
| `sofia_3bodychor462/sol/sol446.txt` | `d883c92c994f78fe29c4bd81ed5d4d95499a795d173b93269c4ec6d21609d430` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol446.txt |
| `sofia_3bodychor462/sol/sol447.txt` | `2116375654ade2fd691437e2d55088c280fef2c9021de4e02a754331275df63e` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol447.txt |
| `sofia_3bodychor462/sol/sol448.txt` | `467a9d15bfcd8b2073fcf098b3abe5a34ff6d8b5b5fc2775889af8658b341837` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol448.txt |
| `sofia_3bodychor462/sol/sol449.txt` | `77ab0888e5e116b5faba67db5e52ec8e01579ddc046445b0cb6c356589e76377` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol449.txt |
| `sofia_3bodychor462/sol/sol450.txt` | `8c6f5dbf8a22d8063973b2e375a5b28c83e2853987da38ebc232b2808ad58e59` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol450.txt |
| `sofia_3bodychor462/sol/sol451.txt` | `fab2bf5fbf111251f5db98beb1f0f186e4820a12723db18028f96e65347aa7d9` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol451.txt |
| `sofia_3bodychor462/sol/sol452.txt` | `0c02d9df5e403fdf52ac9925b4095510268f6e68a1cff60e313e4e2c823eeb8a` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol452.txt |
| `sofia_3bodychor462/sol/sol453.txt` | `bd20699a72c3e701a522337b1292d46ceb39ef1c4880bd67a1131f45c80b93ef` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol453.txt |
| `sofia_3bodychor462/sol/sol454.txt` | `f172009475ad77f557d6fccf18f250c383a4d2a50dcc5d6ed4e4c14ec7f1d2dd` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol454.txt |
| `sofia_3bodychor462/sol/sol455.txt` | `1704299ee4ec5ce8e4ce4ad6e4ffe6b7424e15f1a848f73086f2fdb02b92c6f8` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol455.txt |
| `sofia_3bodychor462/sol/sol456.txt` | `48e80ade0516cb134453b5830a0c345bb1b2423465306276162f8e551577a15f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol456.txt |
| `sofia_3bodychor462/sol/sol457.txt` | `fb4e7caecf7b1d3a3ae190a1ee93399bbafd913a581fec26dd1e3b610e20c7e6` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol457.txt |
| `sofia_3bodychor462/sol/sol458.txt` | `35c33fb16720a80b68a73c1dd04227c6e49e413fa0ad1914cd6b2617127db638` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol458.txt |
| `sofia_3bodychor462/sol/sol459.txt` | `4e556efa30d0a831a6e117116bbcbbe8b34cafe062cba75df614615ca02b1a21` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol459.txt |
| `sofia_3bodychor462/sol/sol460.txt` | `69c1b73fc02f17a88d727fee41166eb7fde6a859667b7a15586317bc8e3d8d5f` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol460.txt |
| `sofia_3bodychor462/sol/sol461.txt` | `6793597b72a6f15a5d2ca4e6da33c87c1bab8f5d444a8d021eb8878057164abe` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol461.txt |
| `sofia_3bodychor462/sol/sol462.txt` | `544e6663d8ce81ac57724fd88025370cb2c32aa45fecda769b4eefdd08e84345` | https://db2.fmi.uni-sofia.bg/3bodychor462/datafiles/sol462.txt |

## hristov2024nc

- **Authors:** Ivan Hristov, Radoslava Hristova, Igor Puzynin, Taisya Puzynina, Zarif Sharipov, Zafar Tukhliev
- **Title:** *Searching for New Nontrivial Choreographies for the Planar Three-Body Problem*
- **Journal:** Physics of Particles and Nuclei 55(3), 495–497 (2024)
- **DOI:** https://doi.org/10.1134/S1063779624030444
- **arXiv:** none found
- **Database:** https://db2.fmi.uni-sofia.bg/3bodychor164/
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated as open: Crossref lists only Springer Nature's text-and-data-mining licence
- published by the authors; included as test data with citation; not ours
- **Files (2):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodychor164/nontriv_180.txt` | `174d6b7651fb74e2bf7b92af1df76f5d954f08a57b69ef54cac4c4bb92b845b7` | https://db2.fmi.uni-sofia.bg/3bodychor164/nontriv_180.txt |
| `sofia_3bodychor164/triv_180.txt` | `5e8dc615b02f4f3fff8e29fb1d39e1fe6421e958b1cf78abfec795de6bbed7e7` | https://db2.fmi.uni-sofia.bg/3bodychor164/triv_180.txt |

## hristov2026

- **Authors:** Ivan Hristov, Radoslava Hristova, Kiyotaka Tanikawa
- **Title:** *An extensive search for stable periodic orbits of the equal-mass zero angular momentum three-body problem*
- **Journal:** New Astronomy 125, 102528 (July 2026)
- **DOI:** https://doi.org/10.1016/j.newast.2026.102528
- **arXiv:** arXiv:2510.22802
- **Database:** https://db2.fmi.uni-sofia.bg/3bodystab971/
- **Fetched:** 27 Oct 2026
- **The paper's licence:** CC BY 4.0 (Crossref: http://creativecommons.org/licenses/by/4.0/)
- published by the authors; included as test data with citation; not ours
- **Files (6):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `sofia_3bodystab971/ics_971_100.txt` | `51695913b9df73996e2108c8106605beb4b3163f2edf9e477691196ff655d76c` | https://db2.fmi.uni-sofia.bg/3bodystab971/ics_971_100.txt |
| `sofia_3bodystab971/ics_971_100_num.txt` | `6da04773f02fb23504a64e5dd41efb3f09c7a0c60d5cfceccba477ac1f22cdb6` | https://db2.fmi.uni-sofia.bg/3bodystab971/ics_971_100_num.txt |
| `sofia_3bodystab971/Shubart_S_num.txt` | `f1e3923a3b67c59cab355e629f2f9be45e1490b963cd26f742340acf7e9ab6c5` | https://db2.fmi.uni-sofia.bg/3bodystab971/Shubart_S_num.txt |
| `sofia_3bodystab971/Intermediate_Moth_num.txt` | `4e24a45d21e0068220d0274d0fca49684ca71debe732bea1f73e683f2bc84846` | https://db2.fmi.uni-sofia.bg/3bodystab971/Intermediate_Moth_num.txt |
| `sofia_3bodystab971/Eight_num.txt` | `24603c8b1f46fe43377aac8a834ce288367bc01da687bfbf7d0ab49ed9570df6` | https://db2.fmi.uni-sofia.bg/3bodystab971/Eight_num.txt |
| `sofia_3bodystab971/Broucke_num.txt` | `8aabec365b38a6ba2610198becafe0e861512f8d66a4b243c2ed153ebbf66bb4` | https://db2.fmi.uni-sofia.bg/3bodystab971/Broucke_num.txt |

## simo2001

- **Authors:** Carles Simó
- **Credit:** Simó's 345 choreographies; the same orbits also reach this folder as Paul Masson's file
  `analyticphysics/simo-initial-conditions.txt` (§ Masson, below), credited to both.
- **Title:** *Dynamical properties of the figure eight solution of the three-body problem*
- **Journal:** Contemporary Mathematics 292, 209–228 (2002)
- **DOI:** https://doi.org/10.1090/conm/292/04926
- **arXiv:** none
- **Database:** http://www.maia.ub.es/dsg/3body.html (the archive `http://www.maia.ub.es/dsg/choreo3.tar.gz`; 345 choreographies, plot data at 4 digits)
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated: Crossref lists none; the archive's own `README` carries no terms
- published by the authors; included as test data with citation; not ours
- **Files (1):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `simo_choreo3/choreo3.tar.gz` | `7d67310fdb599f4f7b55e202bca4581b2fe363b9f948879c06ebca4518befa98` | http://www.maia.ub.es/dsg/choreo3.tar.gz |

## Masson (not in CITATIONS.md)

- **Authors:** Paul Masson (the file and the page)
- **Credit:** the orbits are Carles Simó's (`simo2001` above), which Masson received from him, as his page says; the file
  and its presentation are Masson's. Cite both: Simó for the choreographies, Masson for this file.
- **Title:** *Simó's Three-Body Choreographies in Action (web page, Analytic Physics)*
- **Journal:** none: a web page, not a publication
- **DOI:** none
- **arXiv:** none
- **Database:** https://analyticphysics.com/Gravitational%20Choreographies/Sim%C3%B3's%20Three-Body%20Choreographies%20in%20Action.htm
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated
- published by the authors; included as test data with citation; not ours
- **Files (1):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `analyticphysics/simo-initial-conditions.txt` | `85912a46aaf16600deb7e6c78c69fd6c9108f2a7689c1862117a026ae0e7ce15` | https://analyticphysics.com/www/data/simo-initial-conditions.txt |

## liliao2017

- **Authors:** Xiaoming Li, Shijun Liao
- **Title:** *More than six hundred new families of Newtonian periodic planar collisionless three-body orbits*
- **Journal:** Science China Physics, Mechanics & Astronomy 60, 129511 (2017)
- **DOI:** https://doi.org/10.1007/s11433-017-9078-5
- **arXiv:** arXiv:1705.00527
- **Database:** GitHub `sjtu-liao/three-body` at commit `b96b0193579a9952299d710ed616f529bb7cf898` (no LICENSE file: the API answers 404); https://numericaltank.sjtu.edu.cn/three-body/three-body-movies.htm
- **Fetched:** 27 Oct 2026
- **The paper's licence:** none stated as open: Crossref lists only Springer's text-and-data-mining licence
- published by the authors; included as test data with citation; not ours
- **Files (2):**

| File (under `data/`) | sha256 | Fetched from |
|---|---|---|
| `liliao_github/three-body-pictures.md` | `f77f7e4f6796c28551c8a5da1cc840174532bcab4cf56fc55ecd55e38a7dea4f` | https://raw.githubusercontent.com/sjtu-liao/three-body/b96b0193579a9952299d710ed616f529bb7cf898/three-body-pictures.md |
| `liliao_site/three-body-movies.htm` | `87be5962205182bf779149a1ced09408adee886ee88f8119363506de524dc803` | https://numericaltank.sjtu.edu.cn/three-body/three-body-movies.htm |

## Checking the committed files

From this folder: `cd data && tail -n +2 ../sources.tsv | awk -F'\t' '{print $3"  "$1}' | shasum -a 256 -c`
checks every committed file against its recorded sha256. `bash fetch.sh <empty dir>` stays the provenance
check: it shows that the sources still serve the same bytes.
