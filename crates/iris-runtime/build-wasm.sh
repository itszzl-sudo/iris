#!/bin/bash
# WASM 编译和打包脚本
# 
# 使用方法:
#   ./build-wasm.sh          # 生产构建
#   ./build-wasm.sh dev      # 开发构建

set -e

BUILD_MODE=${1:-release}

echo "🔧 Building iris-runtime WASM module..."
echo "   Mode: $BUILD_MODE"
echo ""

# 编译 WASM
if [ "$BUILD_MODE" = "dev" ]; then
  wasm-pack build --target nodejs --dev
else
  wasm-pack build --target nodejs --release
fi

echo ""
echo "✅ WASM build complete!"
echo ""

# 显示文件大小
if [ -f "pkg/iris_runtime_bg.wasm" ]; then
  WASM_SIZE=$(wc -c < pkg/iris_runtime_bg.wasm)
  echo "📦 WASM binary size: $(numfmt --to=iec-i --suffix=B $WASM_SIZE)"
fi

# 显示生成的文件
echo ""
echo "📁 Generated files:"
ls -lh pkg/ | grep -E "\.(wasm|js|d.ts)$"

echo ""
echo "📝 Next steps:"
echo "   npm install      # Install dependencies"
echo "   npm pack         # Create npm package"
echo "   npm publish      # Publish to npm"
echo ""
