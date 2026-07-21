#include <algorithm>
#include <cstdlib>
#include <memory>
#include <string>
#include <vector>

#include "llvm/ADT/SmallVector.h"
#include "llvm/Analysis/LoopInfo.h"
#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Dominators.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instruction.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IRReader/IRReader.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/JSON.h"
#include "llvm/Support/MD5.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/Support/raw_ostream.h"

namespace {

llvm::cl::opt<std::string> Input(llvm::cl::Positional,
                                 llvm::cl::desc("<input IR>"),
                                 llvm::cl::Required);
llvm::cl::opt<std::string> FunctionName(
    "function", llvm::cl::desc("Exact LLVM function name"),
    llvm::cl::value_desc("name"), llvm::cl::Required);
llvm::cl::opt<unsigned> SourceLine("line", llvm::cl::desc("Source loop line"),
                                   llvm::cl::value_desc("line"),
                                   llvm::cl::Required);

[[noreturn]] void fail(llvm::StringRef Code, llvm::StringRef Message,
                       int64_t Matches = -1) {
  llvm::json::Object Error{{"status", "declined"},
                           {"code", Code},
                           {"message", Message}};
  if (Matches >= 0)
    Error["matches"] = Matches;
  llvm::errs() << llvm::formatv("{0}\n", llvm::json::Value(std::move(Error)));
  std::exit(2);
}

void collectLoops(llvm::Loop *Loop, std::vector<llvm::Loop *> &Output) {
  Output.push_back(Loop);
  for (llvm::Loop *Child : Loop->getSubLoops())
    collectLoops(Child, Output);
}

std::string fingerprint(const llvm::Function &Function, const llvm::Loop &Loop) {
  llvm::MD5 Hash;
  auto update = [&Hash](llvm::StringRef Value) {
    Hash.update(Value);
    Hash.update("\0");
  };
  update(Function.getName());
  update(std::to_string(Loop.getLoopDepth()));
  for (const llvm::BasicBlock &Block : Function) {
    if (!Loop.contains(&Block))
      continue;
    update(Block.getName());
    for (const llvm::Instruction &Instruction : Block) {
      update(Instruction.getOpcodeName());
      update(std::to_string(Instruction.getNumOperands()));
      update(std::to_string(Instruction.getType()->getTypeID()));
      if (const llvm::DebugLoc &Location = Instruction.getDebugLoc()) {
        update(std::to_string(Location.getLine()));
        update(std::to_string(Location.getCol()));
      }
    }
  }
  llvm::MD5::MD5Result Result;
  Hash.final(Result);
  llvm::SmallString<32> Text;
  llvm::MD5::stringifyResult(Result, Text);
  return std::string(Text);
}

} // namespace

int main(int argc, char **argv) {
  llvm::cl::ParseCommandLineOptions(argc, argv, "WhyVec LLVM loop identity\n");
  if (SourceLine == 0)
    fail("input.invalid_source_line", "source line must be positive");

  llvm::LLVMContext Context;
  llvm::SMDiagnostic Diagnostic;
  std::unique_ptr<llvm::Module> Module =
      llvm::parseIRFile(Input, Diagnostic, Context);
  if (!Module) {
    std::string Detail;
    llvm::raw_string_ostream Stream(Detail);
    Diagnostic.print(argv[0], Stream);
    fail("identity.invalid_ir", Stream.str());
  }
  llvm::Function *Function = Module->getFunction(FunctionName);
  if (!Function)
    fail("identity.function_absent", "selected function is absent from IR");

  llvm::DominatorTree Dominators(*Function);
  llvm::LoopInfo Loops(Dominators);
  std::vector<llvm::Loop *> AllLoops;
  for (llvm::Loop *Loop : Loops)
    collectLoops(Loop, AllLoops);

  std::vector<llvm::Loop *> Matches;
  std::copy_if(AllLoops.begin(), AllLoops.end(), std::back_inserter(Matches),
               [](const llvm::Loop *Loop) {
                 llvm::DebugLoc Location = Loop->getStartLoc();
                 return Location && Location.getLine() == SourceLine;
               });
  if (Matches.empty())
    fail("identity.loop_absent", "no loop starts at the selected debug line", 0);
  if (Matches.size() != 1)
    fail("identity.loop_ambiguous",
         "multiple loops start at the selected function/debug line",
         static_cast<int64_t>(Matches.size()));

  llvm::Loop *Loop = Matches.front();
  llvm::DebugLoc Location = Loop->getStartLoc();
  llvm::json::Object Result{
      {"status", "matched"},
      {"function", FunctionName.getValue()},
      {"line", static_cast<int64_t>(Location.getLine())},
      {"column", static_cast<int64_t>(Location.getCol())},
      {"loop_depth", static_cast<int64_t>(Loop->getLoopDepth())},
      {"block_count", static_cast<int64_t>(Loop->getNumBlocks())},
      {"structural_fingerprint", fingerprint(*Function, *Loop)},
      {"mapping_confidence", "high"},
  };
  llvm::outs() << llvm::formatv("{0}\n", llvm::json::Value(std::move(Result)));
  return 0;
}
