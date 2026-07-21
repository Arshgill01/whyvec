; ModuleID = '/home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-opt-wv_e9d6401afd9613efcfff2e0e/exp_6143450e0b4038c0/delta-0.bc'
source_filename = "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @add_vectors_(ptr noalias noundef captures(none) %0, ptr noundef readonly captures(none) %1, ptr noundef readonly captures(none) %2) local_unnamed_addr #0 !dbg !9 {
  %4 = load i32, ptr %2, align 4, !tbaa !13
  %5 = icmp sgt i32 %4, 0, !dbg !17
  br i1 %5, label %iter.check, label %._crit_edge, !dbg !18

iter.check:                                       ; preds = %3
  %wide.trip.count = zext nneg i32 %4 to i64, !dbg !17
  %min.iters.check = icmp ult i32 %4, 4, !dbg !18
  br i1 %min.iters.check, label %.lr.ph.preheader, label %vector.main.loop.iter.check, !dbg !18

.lr.ph.preheader:                                 ; preds = %vec.epilog.iter.check, %vec.epilog.middle.block, %iter.check
  %indvars.iv.ph = phi i64 [ 0, %iter.check ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec17, %vec.epilog.middle.block ]
  br label %.lr.ph, !dbg !18

vector.main.loop.iter.check:                      ; preds = %iter.check
  %min.iters.check8 = icmp ult i32 %4, 32, !dbg !18
  br i1 %min.iters.check8, label %vec.epilog.ph, label %vector.ph, !dbg !18

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.vec = and i64 %wide.trip.count, 2147483616, !dbg !18
  br label %vector.body, !dbg !18

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ], !dbg !19
  %6 = getelementptr inbounds nuw i32, ptr %1, i64 %index, !dbg !20
  %7 = getelementptr inbounds nuw i8, ptr %6, i64 32, !dbg !20
  %8 = getelementptr inbounds nuw i8, ptr %6, i64 64, !dbg !20
  %9 = getelementptr inbounds nuw i8, ptr %6, i64 96, !dbg !20
  %wide.load = load <8 x i32>, ptr %6, align 4, !dbg !20, !tbaa !13
  %wide.load9 = load <8 x i32>, ptr %7, align 4, !dbg !20, !tbaa !13
  %wide.load10 = load <8 x i32>, ptr %8, align 4, !dbg !20, !tbaa !13
  %wide.load11 = load <8 x i32>, ptr %9, align 4, !dbg !20, !tbaa !13
  %10 = getelementptr inbounds nuw i32, ptr %0, i64 %index, !dbg !21
  %11 = getelementptr inbounds nuw i8, ptr %10, i64 32, !dbg !22
  %12 = getelementptr inbounds nuw i8, ptr %10, i64 64, !dbg !22
  %13 = getelementptr inbounds nuw i8, ptr %10, i64 96, !dbg !22
  %wide.load12 = load <8 x i32>, ptr %10, align 4, !dbg !22, !tbaa !13
  %wide.load13 = load <8 x i32>, ptr %11, align 4, !dbg !22, !tbaa !13
  %wide.load14 = load <8 x i32>, ptr %12, align 4, !dbg !22, !tbaa !13
  %wide.load15 = load <8 x i32>, ptr %13, align 4, !dbg !22, !tbaa !13
  %14 = add nsw <8 x i32> %wide.load12, %wide.load, !dbg !22
  %15 = add nsw <8 x i32> %wide.load13, %wide.load9, !dbg !22
  %16 = add nsw <8 x i32> %wide.load14, %wide.load10, !dbg !22
  %17 = add nsw <8 x i32> %wide.load15, %wide.load11, !dbg !22
  store <8 x i32> %14, ptr %10, align 4, !dbg !22, !tbaa !13
  store <8 x i32> %15, ptr %11, align 4, !dbg !22, !tbaa !13
  store <8 x i32> %16, ptr %12, align 4, !dbg !22, !tbaa !13
  store <8 x i32> %17, ptr %13, align 4, !dbg !22, !tbaa !13
  %index.next = add nuw i64 %index, 32, !dbg !19
  %18 = icmp eq i64 %index.next, %n.vec, !dbg !19
  br i1 %18, label %middle.block, label %vector.body, !dbg !19, !llvm.loop !23

middle.block:                                     ; preds = %vector.body
  %cmp.n = icmp eq i64 %n.vec, %wide.trip.count, !dbg !18
  br i1 %cmp.n, label %._crit_edge, label %vec.epilog.iter.check, !dbg !18

vec.epilog.iter.check:                            ; preds = %middle.block
  %n.vec.remaining = and i64 %wide.trip.count, 28, !dbg !18
  %min.epilog.iters.check = icmp eq i64 %n.vec.remaining, 0, !dbg !18
  br i1 %min.epilog.iters.check, label %.lr.ph.preheader, label %vec.epilog.ph, !dbg !18

vec.epilog.ph:                                    ; preds = %vec.epilog.iter.check, %vector.main.loop.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec17 = and i64 %wide.trip.count, 2147483644, !dbg !18
  br label %vec.epilog.vector.body, !dbg !18

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index18 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next21, %vec.epilog.vector.body ], !dbg !19
  %19 = getelementptr inbounds nuw i32, ptr %1, i64 %index18, !dbg !20
  %wide.load19 = load <4 x i32>, ptr %19, align 4, !dbg !20, !tbaa !13
  %20 = getelementptr inbounds nuw i32, ptr %0, i64 %index18, !dbg !21
  %wide.load20 = load <4 x i32>, ptr %20, align 4, !dbg !22, !tbaa !13
  %21 = add nsw <4 x i32> %wide.load20, %wide.load19, !dbg !22
  store <4 x i32> %21, ptr %20, align 4, !dbg !22, !tbaa !13
  %index.next21 = add nuw i64 %index18, 4, !dbg !19
  %22 = icmp eq i64 %index.next21, %n.vec17, !dbg !19
  br i1 %22, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !dbg !19, !llvm.loop !28

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %cmp.n22 = icmp eq i64 %n.vec17, %wide.trip.count, !dbg !18
  br i1 %cmp.n22, label %._crit_edge, label %.lr.ph.preheader, !dbg !18

._crit_edge:                                      ; preds = %.lr.ph, %middle.block, %vec.epilog.middle.block, %3
  ret void, !dbg !29

.lr.ph:                                           ; preds = %.lr.ph.preheader, %.lr.ph
  %indvars.iv = phi i64 [ %indvars.iv.next, %.lr.ph ], [ %indvars.iv.ph, %.lr.ph.preheader ]
  %23 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv, !dbg !20
  %24 = load i32, ptr %23, align 4, !dbg !20, !tbaa !13
  %25 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv, !dbg !21
  %26 = load i32, ptr %25, align 4, !dbg !22, !tbaa !13
  %27 = add nsw i32 %26, %24, !dbg !22
  store i32 %27, ptr %25, align 4, !dbg !22, !tbaa !13
  %indvars.iv.next = add nuw nsw i64 %indvars.iv, 1, !dbg !19
  %exitcond.not = icmp eq i64 %indvars.iv.next, %wide.trip.count, !dbg !17
  br i1 %exitcond.not, label %._crit_edge, label %.lr.ph, !dbg !18, !llvm.loop !30
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
!13 = !{!14, !14, i64 0}
!14 = !{!"int", !15, i64 0}
!15 = !{!"omnipotent char", !16, i64 0}
!16 = !{!"Simple C/C++ TBAA"}
!17 = !DILocation(line: 5, column: 21, scope: !9)
!18 = !DILocation(line: 5, column: 3, scope: !9)
!19 = !DILocation(line: 5, column: 31, scope: !9)
!20 = !DILocation(line: 6, column: 18, scope: !9)
!21 = !DILocation(line: 6, column: 5, scope: !9)
!22 = !DILocation(line: 6, column: 15, scope: !9)
!23 = distinct !{!23, !18, !24, !25, !26, !27}
!24 = !DILocation(line: 6, column: 25, scope: !9)
!25 = !{!"llvm.loop.mustprogress"}
!26 = !{!"llvm.loop.isvectorized", i32 1}
!27 = !{!"llvm.loop.unroll.runtime.disable"}
!28 = distinct !{!28, !18, !24, !25, !26, !27}
!29 = !DILocation(line: 7, column: 1, scope: !9)
!30 = distinct !{!30, !18, !24, !25, !27, !26}
