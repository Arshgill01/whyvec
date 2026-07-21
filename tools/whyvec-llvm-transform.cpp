#include <cstdlib>
#include <memory>
#include <string>

#include "llvm/Bitcode/BitcodeWriter.h"
#include "llvm/IR/Argument.h"
#include "llvm/IR/Attributes.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Type.h"
#include "llvm/IR/Verifier.h"
#include "llvm/IRReader/IRReader.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/FileSystem.h"
#include "llvm/Support/JSON.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/Support/raw_ostream.h"

namespace {

llvm::cl::opt<std::string> Input(llvm::cl::Positional,
                                 llvm::cl::desc("<input IR>"),
                                 llvm::cl::Required);
llvm::cl::opt<std::string> Output("output", llvm::cl::desc("Output bitcode"),
                                  llvm::cl::value_desc("path"),
                                  llvm::cl::Required);
llvm::cl::opt<std::string> FunctionName(
    "function", llvm::cl::desc("Exact LLVM function name"),
    llvm::cl::value_desc("name"), llvm::cl::Required);
llvm::cl::opt<unsigned> ParameterIndex(
    "parameter-index", llvm::cl::desc("Zero-based function parameter index"),
    llvm::cl::value_desc("index"), llvm::cl::Required);

[[noreturn]] void fail(llvm::StringRef Code, llvm::StringRef Message) {
  llvm::json::Object Error{{"status", "declined"},
                           {"code", Code},
                           {"message", Message}};
  llvm::errs() << llvm::formatv("{0:2}\n", llvm::json::Value(std::move(Error)));
  std::exit(2);
}

} // namespace

int main(int argc, char **argv) {
  llvm::cl::ParseCommandLineOptions(argc, argv,
                                    "WhyVec typed LLVM intervention\n");

  llvm::LLVMContext Context;
  llvm::SMDiagnostic Diagnostic;
  std::unique_ptr<llvm::Module> Module =
      llvm::parseIRFile(Input, Diagnostic, Context);
  if (!Module) {
    std::string Detail;
    llvm::raw_string_ostream Stream(Detail);
    Diagnostic.print(argv[0], Stream);
    fail("variant.invalid_ir", Stream.str());
  }

  llvm::Function *Function = Module->getFunction(FunctionName);
  if (!Function)
    fail("identity.function_absent", "selected function is absent from IR");
  if (ParameterIndex >= Function->arg_size())
    fail("variant.parameter_absent",
         "selected parameter index is outside the function signature");

  llvm::Argument *Argument = Function->getArg(ParameterIndex);
  if (!Argument->getType()->isPointerTy())
    fail("variant.parameter_not_pointer",
         "parameter-level noalias requires an LLVM pointer argument");
  if (Argument->hasAttribute(llvm::Attribute::NoAlias))
    fail("variant.assumption_already_present",
         "selected parameter already has the noalias attribute");

  Argument->addAttr(llvm::Attribute::NoAlias);
  if (llvm::verifyModule(*Module, &llvm::errs()))
    fail("variant.verifier_failed", "LLVM rejected the transformed module");

  std::error_code Error;
  llvm::raw_fd_ostream OutputStream(Output, Error, llvm::sys::fs::OF_None);
  if (Error)
    fail("variant.output_failed", Error.message());
  llvm::WriteBitcodeToFile(*Module, OutputStream);
  OutputStream.flush();
  if (OutputStream.has_error())
    fail("variant.output_failed", "failed while writing transformed bitcode");

  llvm::json::Object Result{
      {"status", "applied"},
      {"intervention", "llvm.parameter.noalias"},
      {"function", FunctionName.getValue()},
      {"parameter_index", static_cast<int64_t>(ParameterIndex.getValue())},
      {"before", false},
      {"after", true},
      {"verifier", "passed"},
  };
  llvm::outs() << llvm::formatv("{0}\n", llvm::json::Value(std::move(Result)));
  return 0;
}
