// Apple Vision OCR of a page image. Prints one line per recognised text box:
// y_top x_left x_right <tab> text   (coordinates normalised 0..1, y from top)
import Foundation
import Vision
import AppKit
let args = CommandLine.arguments
guard args.count >= 2, let img = NSImage(contentsOfFile: args[1]),
      let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else { print("usage: ocr <png>"); exit(1) }
let req = VNRecognizeTextRequest()
req.recognitionLevel = .accurate
req.usesLanguageCorrection = false
if args.count >= 3 { req.recognitionLanguages = [args[2]] }
let h = VNImageRequestHandler(cgImage: cg, options: [:])
try h.perform([req])
var rows: [(Double, Double, Double, String)] = []
for o in req.results ?? [] {
  guard let t = o.topCandidates(1).first else { continue }
  let b = o.boundingBox
  rows.append((1 - Double(b.midY), Double(b.minX), Double(b.height), t.string))
}
rows.sort { abs($0.0 - $1.0) < 0.004 ? $0.1 < $1.1 : $0.0 < $1.0 }
for r in rows { print(String(format: "%.4f %.4f %.4f\t", r.0, r.1, r.2) + r.3) }
