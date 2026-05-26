#!/bin/bash

# Harness-Kit 项目初始化脚本

PROJECT_NAME=$1

if [ -z "$PROJECT_NAME" ]; then
    echo "使用方法: bash harness-kit/scripts/new_project.sh <项目名称>"
    exit 1
fi

echo "正在为项目 [$PROJECT_NAME] 初始化 Harness 环境..."

# 创建基础目录结构
mkdir -p sessions/"$PROJECT_NAME"/debate/session_001
mkdir -p sessions/conductor/tasks
mkdir -p product/prd
mkdir -p product/src
mkdir -p records

# 如果项目根目录没有 session_context.md，则从模板复制一个
if [ ! -f "sessions/$PROJECT_NAME/session_context.md" ]; then
    cp harness-kit/core/session_context.template.md sessions/"$PROJECT_NAME"/session_context.md
    echo "✅ 已在 sessions/$PROJECT_NAME/ 目录下生成 session_context.md 模板，请填写后启动流程。"
else
    echo "⚠️ sessions/$PROJECT_NAME/session_context.md 已存在，跳过复制。"
fi

# 创建初始 progress.md
if [ ! -f "sessions/conductor/progress.md" ]; then
    cat <<EOF > sessions/conductor/progress.md
# Conductor Progress

## 当前状态
STATE: INIT
当前 Task: None
更新时间: $(date)

## 已完成 Tasks
- [ ] 尚未开始

## 待执行 Task 队列
- [ ] 等待启动协议完成...

EOF
    echo "✅ 已在 sessions/conductor/ 下生成初始 progress.md。"
fi

echo "🚀 初始化完成！现在请让 Agent 读取 harness-kit/.agent/workflows/onboarding.md 开始工作。"
