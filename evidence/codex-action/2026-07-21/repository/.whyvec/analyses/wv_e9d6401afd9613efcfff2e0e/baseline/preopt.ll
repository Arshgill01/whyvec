; ModuleID = '/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c'
source_filename = "/home/arshdeepsingh/work/github/whyvec/evidence/codex-action/2026-07-21/repository/bound-alias/kernel.c"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-linux-gnu"

; Function Attrs: nounwind uwtable
define dso_local void @add_vectors_(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 !dbg !9 {
  %4 = alloca ptr, align 8
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  store ptr %0, ptr %4, align 8, !tbaa !13
  store ptr %1, ptr %5, align 8, !tbaa !13
  store ptr %2, ptr %6, align 8, !tbaa !13
  call void @llvm.lifetime.start.p0(i64 4, ptr %7) #2, !dbg !18
  store i32 0, ptr %7, align 4, !dbg !19, !tbaa !20
  br label %8, !dbg !18

8:                                                ; preds = %26, %3
  %9 = load i32, ptr %7, align 4, !dbg !22, !tbaa !20
  %10 = load ptr, ptr %6, align 8, !dbg !23, !tbaa !13
  %11 = load i32, ptr %10, align 4, !dbg !24, !tbaa !20
  %12 = icmp slt i32 %9, %11, !dbg !25
  br i1 %12, label %14, label %13, !dbg !26

13:                                               ; preds = %8
  call void @llvm.lifetime.end.p0(i64 4, ptr %7) #2, !dbg !26
  br label %29

14:                                               ; preds = %8
  %15 = load ptr, ptr %5, align 8, !dbg !27, !tbaa !13
  %16 = load i32, ptr %7, align 4, !dbg !28, !tbaa !20
  %17 = sext i32 %16 to i64, !dbg !27
  %18 = getelementptr inbounds i32, ptr %15, i64 %17, !dbg !27
  %19 = load i32, ptr %18, align 4, !dbg !27, !tbaa !20
  %20 = load ptr, ptr %4, align 8, !dbg !29, !tbaa !13
  %21 = load i32, ptr %7, align 4, !dbg !30, !tbaa !20
  %22 = sext i32 %21 to i64, !dbg !29
  %23 = getelementptr inbounds i32, ptr %20, i64 %22, !dbg !29
  %24 = load i32, ptr %23, align 4, !dbg !31, !tbaa !20
  %25 = add nsw i32 %24, %19, !dbg !31
  store i32 %25, ptr %23, align 4, !dbg !31, !tbaa !20
  br label %26, !dbg !29

26:                                               ; preds = %14
  %27 = load i32, ptr %7, align 4, !dbg !32, !tbaa !20
  %28 = add nsw i32 %27, 1, !dbg !32
  store i32 %28, ptr %7, align 4, !dbg !32, !tbaa !20
  br label %8, !dbg !26, !llvm.loop !33

29:                                               ; preds = %13
  ret void, !dbg !36
}

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.start.p0(i64 immarg, ptr captures(none)) #1

; Function Attrs: nocallback nofree nosync nounwind willreturn memory(argmem: readwrite)
declare void @llvm.lifetime.end.p0(i64 immarg, ptr captures(none)) #1

attributes #0 = { nounwind uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64-v3" "target-features"="+avx,+avx2,+bmi,+bmi2,+cmov,+crc32,+cx16,+cx8,+f16c,+fma,+fxsr,+lzcnt,+mmx,+movbe,+popcnt,+sahf,+sse,+sse2,+sse3,+sse4.1,+sse4.2,+ssse3,+x87,+xsave" }
attributes #1 = { nocallback nofree nosync nounwind willreturn memory(argmem: readwrite) }
attributes #2 = { nounwind }

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
!14 = !{!"p1 int", !15, i64 0}
!15 = !{!"any pointer", !16, i64 0}
!16 = !{!"omnipotent char", !17, i64 0}
!17 = !{!"Simple C/C++ TBAA"}
!18 = !DILocation(line: 5, column: 8, scope: !9)
!19 = !DILocation(line: 5, column: 12, scope: !9)
!20 = !{!21, !21, i64 0}
!21 = !{!"int", !16, i64 0}
!22 = !DILocation(line: 5, column: 19, scope: !9)
!23 = !DILocation(line: 5, column: 24, scope: !9)
!24 = !DILocation(line: 5, column: 23, scope: !9)
!25 = !DILocation(line: 5, column: 21, scope: !9)
!26 = !DILocation(line: 5, column: 3, scope: !9)
!27 = !DILocation(line: 6, column: 18, scope: !9)
!28 = !DILocation(line: 6, column: 24, scope: !9)
!29 = !DILocation(line: 6, column: 5, scope: !9)
!30 = !DILocation(line: 6, column: 12, scope: !9)
!31 = !DILocation(line: 6, column: 15, scope: !9)
!32 = !DILocation(line: 5, column: 31, scope: !9)
!33 = distinct !{!33, !26, !34, !35}
!34 = !DILocation(line: 6, column: 25, scope: !9)
!35 = !{!"llvm.loop.mustprogress"}
!36 = !DILocation(line: 7, column: 1, scope: !9)
