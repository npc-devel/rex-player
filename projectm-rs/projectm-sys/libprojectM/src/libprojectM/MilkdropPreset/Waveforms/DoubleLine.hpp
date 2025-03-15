#pragma once

#include "Waveforms/LineBase.hpp"

namespace libprojectM {
namespace MilkdropPreset {
namespace Waveforms {

class DoubleLine : public LineBase
{
protected:
    void GenerateVertices(const PresetState& presetState, const PerFrameContext& presetPerFrameContext) override;
};

} // namespace Waveforms
} // namespace MilkdropPreset
} // namespace libprojectM
