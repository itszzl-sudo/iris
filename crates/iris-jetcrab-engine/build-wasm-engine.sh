#!/bin/bash
# ========================================
# iris-jetcrab-engine WASM 构建脚本 (Linux/macOS)
# ========================================
# 用法: ./build-wasm-engine.sh [debug|release]
# ========================================

set -e

MODE=${1:-release}

echo "========================================"
echo "  iris-jetcrab-engine WASM Build"
echo "========================================"
echo ""

PKG_DIR="$(cd "$(dirname "$0")" && pwd)/pkg-engine"

if [ "$MODE" = "release" ]; then
    echo "[1/3] 编译 WASM (release 模式)..."
    wasm-pack build --target web --release --out-dir "$PKG_DIR"
else
    echo "[1/3] 编译 WASM (debug 模式)..."
    wasm-pack build --target web --out-dir "$PKG_DIR"
fi

echo ""
echo "[2/3] 构建完成"
echo ""

# 显示生成文件
echo "生成的文件:"
ls -lh "$PKG_DIR" | awk '{print "  - " $NF " (" $5 ")"}'

echo ""
echo "[3/3] 输出目录: $PKG_DIR"
echo ""
echo "========================================"
echo "  使用示例:"
echo "========================================"
echo ""
echo "import initEngine, { IrisEngine } from './pkg-engine/iris_jetcrab_engine.js';"
echo ""
echo "await initEngine();"
echo "const engine = new IrisEngine();"
echo ""
echo "// 编译 Vue SFC"
echo "const result = engine.compileSfc(source, 'App.vue');"
echo ""
echo "========================================"
echo ""
