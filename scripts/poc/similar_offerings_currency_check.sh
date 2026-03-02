#!/usr/bin/env bash
set -euo pipefail

WEB_URL="${1:-https://dev.decent-cloud.org/dashboard/marketplace}"

happy_js='(async()=>{const r=await fetch("https://dev-api.decent-cloud.org/api/v1/offerings?limit=200&in_stock_only=true");const p=await r.json();const offerings=p.data||[];const byType=new Map();for(const o of offerings){const t=(o.product_type||"").toLowerCase();if(!byType.has(t))byType.set(t,[]);byType.get(t).push(o);}let baselineSample=null;for(const main of offerings){const t=(main.product_type||"").toLowerCase();const pool=(byType.get(t)||[]).filter(o=>o.id!==main.id);const selected=pool.slice(0,4);const currencies=[...new Set(selected.map(o=>(o.currency||"").toUpperCase()).filter(Boolean))];if(selected.length>=2&&currencies.length>1){baselineSample={mainId:main.id,mainCurrency:main.currency,mainType:main.product_type,similarIds:selected.map(o=>o.id),similarCurrencies:selected.map(o=>o.currency),uniqueCurrencies:currencies};break;}}let fixedViolations=0;for(const main of offerings){const t=(main.product_type||"").toLowerCase();const c=(main.currency||"").toUpperCase();const selected=(byType.get(t)||[]).filter(o=>o.id!==main.id&&(o.currency||"").toUpperCase()===c).slice(0,4);const currencies=[...new Set(selected.map(o=>(o.currency||"").toUpperCase()).filter(Boolean))];if(currencies.length>1)fixedViolations++;}return {count:offerings.length,baselineMixedFound:Boolean(baselineSample),baselineSample,fixedViolations,fixedRuleConsistent:fixedViolations===0};})()'

error_js='(async()=>{const r=await fetch("https://dev-api.decent-cloud.org/api/v1/offerings_DOES_NOT_EXIST");return {status:r.status,ok:r.ok};})()'

echo "[PoC] Happy path: baseline mixed-currency detection + fixed-rule consistency"
node scripts/browser.js eval "$WEB_URL" "$happy_js"

echo "[PoC] Error path: invalid offerings endpoint returns non-OK"
node scripts/browser.js eval "$WEB_URL" "$error_js"
