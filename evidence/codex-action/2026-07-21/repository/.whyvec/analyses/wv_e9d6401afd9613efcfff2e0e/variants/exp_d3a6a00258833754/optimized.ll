; ModuleID = '/home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-opt-wv_e9d6401afd9613efcfff2e0e/exp_d3a6a00258833754/delta-0.bc'
source_filename = "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @add_vectors_(ptr noundef captures(none) %0, ptr noalias noundef readonly captures(none) %1, ptr noundef readonly captures(none) %2) local_unnamed_addr #0 !dbg !9 {
  %4 = load i32, ptr %2, align 4, !dbg !13, !tbaa !14
  %5 = icmp sgt i32 %4, 0, !dbg !18
  br i1 %5, label %.lr.ph, label %._crit_edge, !dbg !19

._crit_edge:                                      ; preds = %.lr.ph, %3
  ret void, !dbg !20

.lr.ph:                                           ; preds = %3, %.lr.ph
  %indvars.iv = phi i64 [ %indvars.iv.next, %.lr.ph ], [ 0, %3 ]
  %6 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv, !dbg !21
  %7 = load i32, ptr %6, align 4, !dbg !21, !tbaa !14
  %8 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv, !dbg !22
  %9 = load i32, ptr %8, align 4, !dbg !23, !tbaa !14
  %10 = add nsw i32 %9, %7, !dbg !23
  store i32 %10, ptr %8, align 4, !dbg !23, !tbaa !14
  %indvars.iv.next = add nuw nsw i64 %indvars.iv, 1, !dbg !24
  %11 = load i32, ptr %2, align 4, !dbg !13, !tbaa !14
  %12 = sext i32 %11 to i64, !dbg !18
  %13 = icmp slt i64 %indvars.iv.next, %12, !dbg !18
  br i1 %13, label %.lr.ph, label %._crit_edge, !dbg !19, !llvm.loop !25
}

attributes #0 = { nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64-v3" "target-features"="+avx,+avx2,+bmi,+bmi2,+cmov,+crc32,+cx16,+cx8,+f16c,+fma,+fxsr,+lzcnt,+mmx,+movbe,+popcnt,+sahf,+sse,+sse2,+sse3,+sse4.1,+sse4.2,+ssse3,+x87,+xsave" }

!llvm.dbg.cu = !{!0}
!llvm.module.flags = !{!2, !3, !4, !5, !6, !7}
!llvm.ident = !{!8}

!0 = distinct !DICompileUnit(language: DW_LANG_C11, file: !1, producer: "Ubuntu clang version 21.1.8 (6ubuntu1)", isOptimized: true, runtimeVersion: 0, emissionKind: LineTablesOnly, splitDebugInlining: false, nameTableKind: None)
!1 = !DIFile(filename: "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c", directory: "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository", checksumkind: CSK_MD5, checksum: "b1d4fd5e828114f66651646ce82b4f88")
!2 = !{i32 7, !"Dwarf Version", i32 5}
!3 = !{i32 2, !"Debug Info Version", i32 3}
!4 = !{i32 1, !"wchar_size", i32 4}
!5 = !{i32 8, !"PIC Level", i32 2}
!6 = !{i32 7, !"PIE Level", i32 2}
!7 = !{i32 7, !"uwtable", i32 2}
!8 = !{!"Ubuntu clang version 21.1.8 (6ubuntu1)"}
!9 = distinct !DISubprogram(name: "add_vectors_", scope: !10, file: !10, line: 2, type: !11, scopeLine: 2, flags: DIFlagPrototyped | DIFlagAllCallsDescribed, spFlags: DISPFlagDefinition | DISPFlagOptimized, unit: !0)
!10 = !DIFile(filename: "bound-alias/kernel.c", directory: "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository", checksumkind: CSK_MD5, checksum: "b1d4fd5e828114f66651646ce82b4f88")
!11 = !DISubroutineType(types: !12)
!12 = !{}
!13 = !DILocation(line: 5, column: 23, scope: !9)
!14 = !{!15, !15, i64 0}
!15 = !{!"int", !16, i64 0}
!16 = !{!"omnipotent char", !17, i64 0}
!17 = !{!"Simple C/C++ TBAA"}
!18 = !DILocation(line: 5, column: 21, scope: !9)
!19 = !DILocation(line: 5, column: 3, scope: !9)
!20 = !DILocation(line: 7, column: 1, scope: !9)
!21 = !DILocation(line: 6, column: 18, scope: !9)
!22 = !DILocation(line: 6, column: 5, scope: !9)
!23 = !DILocation(line: 6, column: 15, scope: !9)
!24 = !DILocation(line: 5, column: 31, scope: !9)
!25 = distinct !{!25, !19, !26, !27}
!26 = !DILocation(line: 6, column: 25, scope: !9)
!27 = !{!"llvm.loop.mustprogress"}
