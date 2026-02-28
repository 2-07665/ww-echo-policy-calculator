export function createUpgradeScorerConfigView({
  state,
  elements,
  numberOr,
  targetScoreDigits,
  targetScoreStep,
  scorerWuwaEchoTool,
  scorerMcBoostAssistant,
  scorerQqBot,
  mcBoostAssistantLockedMainBuffScore,
  mcBoostAssistantLockedNormalizedMaxScore,
  qqBotLockedNormalizedMaxScore,
  isFixedScorer,
  isMcBoostAssistantScorer,
  isQqBotScorer,
  getScorerConfig,
  setHelpTooltip,
}) {
  function renderScorerConfig() {
    elements.scorerTypeSelect.value = state.scorerType;
    let scorerHelpText = '';
    const scorerTypeLabel = String(
      elements.scorerTypeSelect?.selectedOptions?.[0]?.textContent || state.scorerType,
    ).trim();
    if (elements.scorerConfigHelp) {
      elements.scorerConfigHelp.setAttribute('aria-label', `当前选中模式说明：${scorerTypeLabel}`);
    }

    if (isFixedScorer()) {
      elements.linearParams.hidden = true;
      elements.linearParams.style.display = 'none';
      elements.mainBuffScoreInput.disabled = true;
      elements.normalizedMaxScoreInput.disabled = true;
      scorerHelpText =
        '固定评分模式下，每个词条直接按设置的权重计分，不受词条数值高低影响。\n权重和目标分数只能为整数。\n此评分模式用于只关注词条数目的场景。';
      setHelpTooltip(elements.scorerConfigHelp, scorerHelpText);
      return;
    }

    const config = getScorerConfig();
    elements.linearParams.hidden = false;
    elements.linearParams.style.display = '';

    config.mainBuffScore = Math.max(0, numberOr(config.mainBuffScore, 0));
    config.normalizedMaxScore = Math.max(targetScoreStep, numberOr(config.normalizedMaxScore, 0));

    if (isMcBoostAssistantScorer()) {
      config.mainBuffScore = mcBoostAssistantLockedMainBuffScore;
      config.normalizedMaxScore = mcBoostAssistantLockedNormalizedMaxScore;
      elements.mainBuffScoreInput.disabled = true;
      elements.normalizedMaxScoreInput.disabled = true;
    } else if (isQqBotScorer()) {
      config.normalizedMaxScore = qqBotLockedNormalizedMaxScore;
      elements.mainBuffScoreInput.disabled = false;
      elements.normalizedMaxScoreInput.disabled = true;
    } else {
      elements.mainBuffScoreInput.disabled = false;
      elements.normalizedMaxScoreInput.disabled = false;
    }

    elements.mainBuffScoreInput.value = config.mainBuffScore.toFixed(targetScoreDigits);
    elements.normalizedMaxScoreInput.value = config.normalizedMaxScore.toFixed(targetScoreDigits);

    if (state.scorerType === scorerMcBoostAssistant) {
      scorerHelpText =
        '微信小程序漂泊者强化助手按线性规则评分。归一化总分 120.00，其中主词条固定贡献 50.00。请在小程序中查看词条权重。\n这里的单词条分数为它们对总分的贡献，因而是小程序内显示的0.7倍。并且未计入对生存词条的“安慰分数”。\n评分权重是由词条对期望伤害的提升计算得到，但考虑的是词条对0+1角色与对6+5角色的提升平均值。';
    } else if (state.scorerType === scorerWuwaEchoTool) {
      scorerHelpText =
        'Wuwa Echo Tool 按线性规则评分。\n由于工具原始的评分规则中主词条占比高，且未对单件声骸给出归一化分数，这里等价转换了其提供的权重，以提供针对单件声骸副词条的评分。\n对不同链度的角色提供了不同的权重。';
    } else if (state.scorerType === scorerQqBot) {
      scorerHelpText =
        'WutheringWavesUID （QQ机器人）按线性规则评分，归一化总分 50.00。请使用“ww(角色名)权重”命令查看角色权重，如“ww椿权重”。\n如需总分与机器人结果一致，需在主词条原始分数处填写对应Cost声骸两个主词条的权重与数值乘积之和。\n"当前角色评分标准仅供参考与娱乐，不代表任何官方或权威的评价。"';
    } else {
      scorerHelpText =
        '自定义线性评分模式下，按词条数值占该词条最大数值的比例线性计分。词条权重对应词条最大数值的原始分数。\n可设置声骸主词条的原始分数与归一化总分。权重与分数支持小数。\n建议配合拉表计算得到的词条权重使用。';
    }

    setHelpTooltip(elements.scorerConfigHelp, scorerHelpText);
  }

  return {
    renderScorerConfig,
  };
}
