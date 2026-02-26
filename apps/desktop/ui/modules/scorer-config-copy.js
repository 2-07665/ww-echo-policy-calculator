export function createScorerConfigCopyHelpers({
  state,
  numberOr,
  targetScoreStep,
}) {
  function copyWeightMap(src) {
    const out = {};
    state.buffTypes.forEach((name) => {
      out[name] = numberOr(src?.[name], 0);
    });
    return out;
  }

  function copyScorerConfig(src) {
    const out = {
      weights: copyWeightMap(src?.weights),
    };
    if (Object.prototype.hasOwnProperty.call(src || {}, 'mainBuffScore')) {
      out.mainBuffScore = numberOr(src.mainBuffScore, 0);
    }
    if (Object.prototype.hasOwnProperty.call(src || {}, 'normalizedMaxScore')) {
      out.normalizedMaxScore = numberOr(src.normalizedMaxScore, targetScoreStep);
    }
    return out;
  }

  return {
    copyWeightMap,
    copyScorerConfig,
  };
}
