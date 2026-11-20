// Reading B helper: Apple Vision text recognition of a rendered page image (never the PDF text layer).
// Usage: ocr <image.png>   Prints one line per recognised text box: mid-y, min-x, max-x (normalised, origin
// bottom-left), then the text. Accurate level, language correction off.
import Foundation
import Vision
import AppKit
let path = CommandLine.arguments[1]
guard let img = NSImage(contentsOfFile: path),
      let cg = img.cgImage(forProposedRect: nil, context: nil, hints: nil) else { fatalError("load") }
let req = VNRecognizeTextRequest()
req.recognitionLevel = .accurate
req.usesLanguageCorrection = false
if CommandLine.arguments.count > 2 { req.recognitionLanguages = CommandLine.arguments[2].components(separatedBy: ",") }
req.minimumTextHeight = 0.003
let h = VNImageRequestHandler(cgImage: cg, options: [:])
try h.perform([req])
for o in (req.results ?? []) {
  guard let c = o.topCandidates(1).first else { continue }
  let b = o.boundingBox
  print(String(format: "%.4f\t%.4f\t%.4f\t%@", b.midY, b.minX, b.maxX, c.string))
}
