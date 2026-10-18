use windows::core::{Interface, BSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationValuePattern,
    UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_TextPatternId, UIA_ValuePatternId,
};

/// 方案一：通过 Windows UI Automation 零按键直接读取焦点控件文本
/// 增加严格安全保护：仅对真正的可编辑 Edit 控件生效，坚决拒绝读取整页 Document
pub fn get_focused_text() -> Result<String, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("Failed to create UIAutomation instance: {}", e))?;

        let focused = automation.GetFocusedElement()
            .map_err(|e| format!("GetFocusedElement failed: {}", e))?;

        let ctrl_type = focused.CurrentControlType().map(|c| c.0).unwrap_or(0);

        // 安全防线：密码/支付类输入框一律不读。
        // 否则在登录框、支付页按热键，会把密码原文发给大模型。
        if let Ok(is_pwd) = focused.CurrentIsPassword() {
            if is_pwd.as_bool() {
                return Err("焦点在密码输入框，已跳过（安全检查）".to_string());
            }
        }

        // 安全防线：如果焦点是整张网页文档(50030)或未知窗体，严禁当成输入框！
        if ctrl_type == UIA_DocumentControlTypeId.0 {
            return Err("Focused element is a web Document, not an input box".to_string());
        }

        // 优先检查 ValuePattern（普通输入框）
        if let Ok(pattern_unknown) = focused.GetCurrentPattern(UIA_ValuePatternId) {
            if let Ok(value_pattern) = pattern_unknown.cast::<IUIAutomationValuePattern>() {
                // 如果控件标记为只读，绝不当做聊天输入框
                if let Ok(is_ro) = value_pattern.CurrentIsReadOnly() {
                    if is_ro.as_bool() {
                        return Err("Focused element is read-only".to_string());
                    }
                }
                if let Ok(bstr_val) = value_pattern.CurrentValue() {
                    let s = bstr_val.to_string();
                    if !s.trim().is_empty() {
                        return Ok(s);
                    }
                }
            }
        }

        // 备选检查 TextPattern（仅允许 EditControlTypeId 50004，防止提取整页网页文字）
        if ctrl_type == UIA_EditControlTypeId.0 {
            if let Ok(pattern_unknown) = focused.GetCurrentPattern(UIA_TextPatternId) {
                if let Ok(text_pattern) = pattern_unknown.cast::<IUIAutomationTextPattern>() {
                    if let Ok(range) = text_pattern.DocumentRange() {
                        if let Ok(bstr_val) = range.GetText(-1) {
                            let s = bstr_val.to_string();
                            if !s.trim().is_empty() {
                                return Ok(s);
                            }
                        }
                    }
                }
            }
        }

        Err("Focused element does not provide text via UIA".to_string())
    }
}

/// 方案一：通过 Windows UI Automation 零按键直接将文本设置进焦点输入框
pub fn set_focused_text(new_text: &str) -> Result<(), String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("Failed to create UIAutomation instance: {}", e))?;

        let focused = automation.GetFocusedElement()
            .map_err(|e| format!("GetFocusedElement failed: {}", e))?;

        let ctrl_type = focused.CurrentControlType().map(|c| c.0).unwrap_or(0);
        if ctrl_type == UIA_DocumentControlTypeId.0 {
            return Err("Cannot set text to a Document".to_string());
        }

        // 绝不往密码框里写内容
        if let Ok(is_pwd) = focused.CurrentIsPassword() {
            if is_pwd.as_bool() {
                return Err("焦点在密码输入框，已拒绝写入（安全检查）".to_string());
            }
        }

        let pattern_unknown = focused.GetCurrentPattern(UIA_ValuePatternId)
            .map_err(|e| format!("Element does not support ValuePattern: {}", e))?;

        let value_pattern: IUIAutomationValuePattern = pattern_unknown.cast()
            .map_err(|e| format!("Cast to IUIAutomationValuePattern failed: {}", e))?;

        if let Ok(is_ro) = value_pattern.CurrentIsReadOnly() {
            if is_ro.as_bool() {
                return Err("Focused element is read-only".to_string());
            }
        }

        let bstr = BSTR::from(new_text);
        value_pattern.SetValue(&bstr)
            .map_err(|e| format!("SetValue failed: {}", e))?;

        Ok(())
    }
}
