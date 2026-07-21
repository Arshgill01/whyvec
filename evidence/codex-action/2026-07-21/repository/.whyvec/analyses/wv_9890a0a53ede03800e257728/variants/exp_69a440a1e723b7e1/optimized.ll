; ModuleID = '/home/arshdeepsingh/work/whyvec-validation-tmp/whyvec-opt-wv_9890a0a53ede03800e257728/exp_69a440a1e723b7e1/delta-0.bc'
source_filename = "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nofree norecurse nosync nounwind memory(argmem: readwrite) uwtable
define dso_local void @add_vectors_(ptr noundef captures(none) %0, ptr noundef readonly captures(none) %1, ptr noalias noundef readonly captures(none) %2) local_unnamed_addr #0 !dbg !9 {
  %4 = load i32, ptr %2, align 4, !tbaa !13
  %5 = icmp sgt i32 %4, 0, !dbg !17
  br i1 %5, label %iter.check, label %._crit_edge, !dbg !18

iter.check:                                       ; preds = %3
  %wide.trip.count = zext nneg i32 %4 to i64, !dbg !17
  %min.iters.check = icmp ult i32 %4, 4, !dbg !18
  br i1 %min.iters.check, label %.lr.ph.preheader, label %vector.memcheck, !dbg !18

.lr.ph.preheader:                                 ; preds = %vec.epilog.iter.check, %vec.epilog.middle.block, %vector.memcheck, %iter.check
  %indvars.iv.ph = phi i64 [ 0, %iter.check ], [ 0, %vector.memcheck ], [ %n.vec, %vec.epilog.iter.check ], [ %n.vec18, %vec.epilog.middle.block ]
  %6 = sub nsw i64 %wide.trip.count, %indvars.iv.ph, !dbg !18
  %xtraiter = and i64 %6, 7, !dbg !18
  %lcmp.mod.not = icmp eq i64 %xtraiter, 0, !dbg !18
  br i1 %lcmp.mod.not, label %.lr.ph.prol.loopexit, label %.lr.ph.prol, !dbg !18

.lr.ph.prol:                                      ; preds = %.lr.ph.preheader, %.lr.ph.prol
  %indvars.iv.prol = phi i64 [ %indvars.iv.next.prol, %.lr.ph.prol ], [ %indvars.iv.ph, %.lr.ph.preheader ]
  %prol.iter = phi i64 [ %prol.iter.next, %.lr.ph.prol ], [ 0, %.lr.ph.preheader ]
  %7 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.prol, !dbg !19
  %8 = load i32, ptr %7, align 4, !dbg !19, !tbaa !13
  %9 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.prol, !dbg !20
  %10 = load i32, ptr %9, align 4, !dbg !21, !tbaa !13
  %11 = add nsw i32 %10, %8, !dbg !21
  store i32 %11, ptr %9, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.prol = add nuw nsw i64 %indvars.iv.prol, 1, !dbg !22
  %prol.iter.next = add i64 %prol.iter, 1, !dbg !18
  %prol.iter.cmp.not = icmp eq i64 %prol.iter.next, %xtraiter, !dbg !18
  br i1 %prol.iter.cmp.not, label %.lr.ph.prol.loopexit, label %.lr.ph.prol, !dbg !18, !llvm.loop !23

.lr.ph.prol.loopexit:                             ; preds = %.lr.ph.prol, %.lr.ph.preheader
  %indvars.iv.unr = phi i64 [ %indvars.iv.ph, %.lr.ph.preheader ], [ %indvars.iv.next.prol, %.lr.ph.prol ]
  %12 = sub nsw i64 %indvars.iv.ph, %wide.trip.count, !dbg !18
  %13 = icmp ugt i64 %12, -8, !dbg !18
  br i1 %13, label %._crit_edge, label %.lr.ph, !dbg !18

vector.memcheck:                                  ; preds = %iter.check
  %14 = shl nuw nsw i64 %wide.trip.count, 2, !dbg !18
  %scevgep = getelementptr i8, ptr %0, i64 %14, !dbg !18
  %scevgep8 = getelementptr i8, ptr %1, i64 %14, !dbg !18
  %bound0 = icmp ult ptr %0, %scevgep8, !dbg !18
  %bound1 = icmp ult ptr %1, %scevgep, !dbg !18
  %found.conflict = and i1 %bound0, %bound1, !dbg !18
  br i1 %found.conflict, label %.lr.ph.preheader, label %vector.main.loop.iter.check, !dbg !22

vector.main.loop.iter.check:                      ; preds = %vector.memcheck
  %min.iters.check9 = icmp ult i32 %4, 32, !dbg !18
  br i1 %min.iters.check9, label %vec.epilog.ph, label %vector.ph, !dbg !18

vector.ph:                                        ; preds = %vector.main.loop.iter.check
  %n.vec = and i64 %wide.trip.count, 2147483616, !dbg !18
  br label %vector.body, !dbg !18

vector.body:                                      ; preds = %vector.body, %vector.ph
  %index = phi i64 [ 0, %vector.ph ], [ %index.next, %vector.body ], !dbg !22
  %15 = getelementptr inbounds nuw i32, ptr %1, i64 %index, !dbg !19
  %16 = getelementptr inbounds nuw i8, ptr %15, i64 32, !dbg !19
  %17 = getelementptr inbounds nuw i8, ptr %15, i64 64, !dbg !19
  %18 = getelementptr inbounds nuw i8, ptr %15, i64 96, !dbg !19
  %wide.load = load <8 x i32>, ptr %15, align 4, !dbg !19, !tbaa !13, !alias.scope !25
  %wide.load10 = load <8 x i32>, ptr %16, align 4, !dbg !19, !tbaa !13, !alias.scope !25
  %wide.load11 = load <8 x i32>, ptr %17, align 4, !dbg !19, !tbaa !13, !alias.scope !25
  %wide.load12 = load <8 x i32>, ptr %18, align 4, !dbg !19, !tbaa !13, !alias.scope !25
  %19 = getelementptr inbounds nuw i32, ptr %0, i64 %index, !dbg !20
  %20 = getelementptr inbounds nuw i8, ptr %19, i64 32, !dbg !21
  %21 = getelementptr inbounds nuw i8, ptr %19, i64 64, !dbg !21
  %22 = getelementptr inbounds nuw i8, ptr %19, i64 96, !dbg !21
  %wide.load13 = load <8 x i32>, ptr %19, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %wide.load14 = load <8 x i32>, ptr %20, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %wide.load15 = load <8 x i32>, ptr %21, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %wide.load16 = load <8 x i32>, ptr %22, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %23 = add nsw <8 x i32> %wide.load13, %wide.load, !dbg !21
  %24 = add nsw <8 x i32> %wide.load14, %wide.load10, !dbg !21
  %25 = add nsw <8 x i32> %wide.load15, %wide.load11, !dbg !21
  %26 = add nsw <8 x i32> %wide.load16, %wide.load12, !dbg !21
  store <8 x i32> %23, ptr %19, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  store <8 x i32> %24, ptr %20, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  store <8 x i32> %25, ptr %21, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  store <8 x i32> %26, ptr %22, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %index.next = add nuw i64 %index, 32, !dbg !22
  %27 = icmp eq i64 %index.next, %n.vec, !dbg !22
  br i1 %27, label %middle.block, label %vector.body, !dbg !22, !llvm.loop !30

middle.block:                                     ; preds = %vector.body
  %cmp.n = icmp eq i64 %n.vec, %wide.trip.count, !dbg !18
  br i1 %cmp.n, label %._crit_edge, label %vec.epilog.iter.check, !dbg !18

vec.epilog.iter.check:                            ; preds = %middle.block
  %n.vec.remaining = and i64 %wide.trip.count, 28, !dbg !18
  %min.epilog.iters.check = icmp eq i64 %n.vec.remaining, 0, !dbg !18
  br i1 %min.epilog.iters.check, label %.lr.ph.preheader, label %vec.epilog.ph, !dbg !18

vec.epilog.ph:                                    ; preds = %vec.epilog.iter.check, %vector.main.loop.iter.check
  %vec.epilog.resume.val = phi i64 [ %n.vec, %vec.epilog.iter.check ], [ 0, %vector.main.loop.iter.check ]
  %n.vec18 = and i64 %wide.trip.count, 2147483644, !dbg !18
  br label %vec.epilog.vector.body, !dbg !18

vec.epilog.vector.body:                           ; preds = %vec.epilog.vector.body, %vec.epilog.ph
  %index19 = phi i64 [ %vec.epilog.resume.val, %vec.epilog.ph ], [ %index.next22, %vec.epilog.vector.body ], !dbg !22
  %28 = getelementptr inbounds nuw i32, ptr %1, i64 %index19, !dbg !19
  %wide.load20 = load <4 x i32>, ptr %28, align 4, !dbg !19, !tbaa !13, !alias.scope !25
  %29 = getelementptr inbounds nuw i32, ptr %0, i64 %index19, !dbg !20
  %wide.load21 = load <4 x i32>, ptr %29, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %30 = add nsw <4 x i32> %wide.load21, %wide.load20, !dbg !21
  store <4 x i32> %30, ptr %29, align 4, !dbg !21, !tbaa !13, !alias.scope !28, !noalias !25
  %index.next22 = add nuw i64 %index19, 4, !dbg !22
  %31 = icmp eq i64 %index.next22, %n.vec18, !dbg !22
  br i1 %31, label %vec.epilog.middle.block, label %vec.epilog.vector.body, !dbg !22, !llvm.loop !35

vec.epilog.middle.block:                          ; preds = %vec.epilog.vector.body
  %cmp.n23 = icmp eq i64 %n.vec18, %wide.trip.count, !dbg !18
  br i1 %cmp.n23, label %._crit_edge, label %.lr.ph.preheader, !dbg !18

._crit_edge:                                      ; preds = %.lr.ph.prol.loopexit, %.lr.ph, %middle.block, %vec.epilog.middle.block, %3
  ret void, !dbg !36

.lr.ph:                                           ; preds = %.lr.ph.prol.loopexit, %.lr.ph
  %indvars.iv = phi i64 [ %indvars.iv.next.7, %.lr.ph ], [ %indvars.iv.unr, %.lr.ph.prol.loopexit ]
  %32 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv, !dbg !19
  %33 = load i32, ptr %32, align 4, !dbg !19, !tbaa !13
  %34 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv, !dbg !20
  %35 = load i32, ptr %34, align 4, !dbg !21, !tbaa !13
  %36 = add nsw i32 %35, %33, !dbg !21
  store i32 %36, ptr %34, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next = add nuw nsw i64 %indvars.iv, 1, !dbg !22
  %37 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next, !dbg !19
  %38 = load i32, ptr %37, align 4, !dbg !19, !tbaa !13
  %39 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next, !dbg !20
  %40 = load i32, ptr %39, align 4, !dbg !21, !tbaa !13
  %41 = add nsw i32 %40, %38, !dbg !21
  store i32 %41, ptr %39, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.1 = add nuw nsw i64 %indvars.iv, 2, !dbg !22
  %42 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.1, !dbg !19
  %43 = load i32, ptr %42, align 4, !dbg !19, !tbaa !13
  %44 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.1, !dbg !20
  %45 = load i32, ptr %44, align 4, !dbg !21, !tbaa !13
  %46 = add nsw i32 %45, %43, !dbg !21
  store i32 %46, ptr %44, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.2 = add nuw nsw i64 %indvars.iv, 3, !dbg !22
  %47 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.2, !dbg !19
  %48 = load i32, ptr %47, align 4, !dbg !19, !tbaa !13
  %49 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.2, !dbg !20
  %50 = load i32, ptr %49, align 4, !dbg !21, !tbaa !13
  %51 = add nsw i32 %50, %48, !dbg !21
  store i32 %51, ptr %49, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.3 = add nuw nsw i64 %indvars.iv, 4, !dbg !22
  %52 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.3, !dbg !19
  %53 = load i32, ptr %52, align 4, !dbg !19, !tbaa !13
  %54 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.3, !dbg !20
  %55 = load i32, ptr %54, align 4, !dbg !21, !tbaa !13
  %56 = add nsw i32 %55, %53, !dbg !21
  store i32 %56, ptr %54, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.4 = add nuw nsw i64 %indvars.iv, 5, !dbg !22
  %57 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.4, !dbg !19
  %58 = load i32, ptr %57, align 4, !dbg !19, !tbaa !13
  %59 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.4, !dbg !20
  %60 = load i32, ptr %59, align 4, !dbg !21, !tbaa !13
  %61 = add nsw i32 %60, %58, !dbg !21
  store i32 %61, ptr %59, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.5 = add nuw nsw i64 %indvars.iv, 6, !dbg !22
  %62 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.5, !dbg !19
  %63 = load i32, ptr %62, align 4, !dbg !19, !tbaa !13
  %64 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.5, !dbg !20
  %65 = load i32, ptr %64, align 4, !dbg !21, !tbaa !13
  %66 = add nsw i32 %65, %63, !dbg !21
  store i32 %66, ptr %64, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.6 = add nuw nsw i64 %indvars.iv, 7, !dbg !22
  %67 = getelementptr inbounds nuw i32, ptr %1, i64 %indvars.iv.next.6, !dbg !19
  %68 = load i32, ptr %67, align 4, !dbg !19, !tbaa !13
  %69 = getelementptr inbounds nuw i32, ptr %0, i64 %indvars.iv.next.6, !dbg !20
  %70 = load i32, ptr %69, align 4, !dbg !21, !tbaa !13
  %71 = add nsw i32 %70, %68, !dbg !21
  store i32 %71, ptr %69, align 4, !dbg !21, !tbaa !13
  %indvars.iv.next.7 = add nuw nsw i64 %indvars.iv, 8, !dbg !22
  %exitcond.not.7 = icmp eq i64 %indvars.iv.next.7, %wide.trip.count, !dbg !17
  br i1 %exitcond.not.7, label %._crit_edge, label %.lr.ph, !dbg !18, !llvm.loop !37
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
!19 = !DILocation(line: 6, column: 18, scope: !9)
!20 = !DILocation(line: 6, column: 5, scope: !9)
!21 = !DILocation(line: 6, column: 15, scope: !9)
!22 = !DILocation(line: 5, column: 31, scope: !9)
!23 = distinct !{!23, !24}
!24 = !{!"llvm.loop.unroll.disable"}
!25 = !{!26}
!26 = distinct !{!26, !27}
!27 = distinct !{!27, !"LVerDomain"}
!28 = !{!29}
!29 = distinct !{!29, !27}
!30 = distinct !{!30, !18, !31, !32, !33, !34}
!31 = !DILocation(line: 6, column: 25, scope: !9)
!32 = !{!"llvm.loop.mustprogress"}
!33 = !{!"llvm.loop.isvectorized", i32 1}
!34 = !{!"llvm.loop.unroll.runtime.disable"}
!35 = distinct !{!35, !18, !31, !32, !33, !34}
!36 = !DILocation(line: 7, column: 1, scope: !9)
!37 = distinct !{!37, !18, !31, !32, !33}
