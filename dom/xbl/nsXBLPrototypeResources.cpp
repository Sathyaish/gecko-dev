/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#ifdef MOZ_OLD_STYLE
#include "nsIStyleRuleProcessor.h"
#endif
#include "nsIDocument.h"
#include "nsIContent.h"
#include "nsIServiceManager.h"
#include "nsXBLResourceLoader.h"
#include "nsXBLPrototypeResources.h"
#include "nsXBLPrototypeBinding.h"
#include "nsIDocumentObserver.h"
#include "mozilla/css/Loader.h"
#include "nsIURI.h"
#include "nsLayoutCID.h"
#ifdef MOZ_OLD_STYLE
#include "nsCSSRuleProcessor.h"
#include "nsStyleSet.h"
#endif
#include "mozilla/dom/URL.h"
#include "mozilla/DebugOnly.h"
#include "mozilla/StyleSheet.h"
#include "mozilla/StyleSheetInlines.h"

using namespace mozilla;
using mozilla::dom::IsChromeURI;

nsXBLPrototypeResources::nsXBLPrototypeResources(nsXBLPrototypeBinding* aBinding)
{
  MOZ_COUNT_CTOR(nsXBLPrototypeResources);

  mLoader = new nsXBLResourceLoader(aBinding, this);
}

nsXBLPrototypeResources::~nsXBLPrototypeResources()
{
  MOZ_COUNT_DTOR(nsXBLPrototypeResources);
  if (mLoader) {
    mLoader->mResources = nullptr;
  }
}

void
nsXBLPrototypeResources::AddResource(nsAtom* aResourceType, const nsAString& aSrc)
{
  if (mLoader)
    mLoader->AddResource(aResourceType, aSrc);
}

bool
nsXBLPrototypeResources::LoadResources(nsIContent* aBoundElement)
{
  if (mLoader) {
    return mLoader->LoadResources(aBoundElement);
  }

  return true; // All resources loaded.
}

void
nsXBLPrototypeResources::AddResourceListener(nsIContent* aBoundElement)
{
  if (mLoader)
    mLoader->AddResourceListener(aBoundElement);
}

nsresult
nsXBLPrototypeResources::FlushSkinSheets()
{
  if (mStyleSheetList.Length() == 0)
    return NS_OK;

  nsCOMPtr<nsIDocument> doc =
    mLoader->mBinding->XBLDocumentInfo()->GetDocument();

  // If doc is null, we're in the process of tearing things down, so just
  // return without rebuilding anything.
  if (!doc) {
    return NS_OK;
  }

  // We have scoped stylesheets.  Reload any chrome stylesheets we
  // encounter.  (If they aren't skin sheets, it doesn't matter, since
  // they'll still be in the chrome cache.  Skip inline sheets, which
  // skin sheets can't be, and which in any case don't have a usable
  // URL to reload.)

  nsTArray<RefPtr<StyleSheet>> oldSheets;

  oldSheets.SwapElements(mStyleSheetList);

  mozilla::css::Loader* cssLoader = doc->CSSLoader();

  for (size_t i = 0, count = oldSheets.Length(); i < count; ++i) {
    StyleSheet* oldSheet = oldSheets[i];

    nsIURI* uri = oldSheet->GetSheetURI();

    RefPtr<StyleSheet> newSheet;
    if (!oldSheet->IsInline() && IsChromeURI(uri)) {
      if (NS_FAILED(cssLoader->LoadSheetSync(uri, &newSheet)))
        continue;
    }
    else {
      newSheet = oldSheet;
    }

    mStyleSheetList.AppendElement(newSheet);
  }

  if (doc->IsStyledByServo()) {
    // There may be no shell during unlink.
    //
    // FIXME(emilio): We shouldn't skip shadow root style updates just because?
    // Though during unlink is fine I guess...
    if (auto* shell = doc->GetShell()) {
      MOZ_ASSERT(shell->GetPresContext());
      ComputeServoStyles(*shell->StyleSet()->AsServo());
    }
  } else {
#ifdef MOZ_OLD_STYLE
    GatherRuleProcessor();
#else
    MOZ_CRASH("old style system disabled");
#endif
  }

  return NS_OK;
}

nsresult
nsXBLPrototypeResources::Write(nsIObjectOutputStream* aStream)
{
  if (mLoader)
    return mLoader->Write(aStream);
  return NS_OK;
}

void
nsXBLPrototypeResources::Traverse(nsCycleCollectionTraversalCallback &cb)
{
  NS_CYCLE_COLLECTION_NOTE_EDGE_NAME(cb, "proto mResources mLoader");
  cb.NoteXPCOMChild(mLoader);

#ifdef MOZ_OLD_STYLE
  CycleCollectionNoteChild(cb, mRuleProcessor.get(), "mRuleProcessor");
#endif
  ImplCycleCollectionTraverse(cb, mStyleSheetList, "mStyleSheetList");
}

void
nsXBLPrototypeResources::Unlink()
{
  mStyleSheetList.Clear();
#ifdef MOZ_OLD_STYLE
  mRuleProcessor = nullptr;
#endif
}

void
nsXBLPrototypeResources::ClearLoader()
{
  mLoader = nullptr;
}

#ifdef MOZ_OLD_STYLE
void
nsXBLPrototypeResources::GatherRuleProcessor()
{
  nsTArray<RefPtr<CSSStyleSheet>> sheets(mStyleSheetList.Length());
  for (StyleSheet* sheet : mStyleSheetList) {
    MOZ_ASSERT(sheet->IsGecko(),
               "GatherRuleProcessor must only be called for "
               "nsXBLPrototypeResources objects with Gecko-flavored style "
               "backends");
    sheets.AppendElement(sheet->AsGecko());
  }
  mRuleProcessor = new nsCSSRuleProcessor(Move(sheets),
                                          SheetType::Doc,
                                          nullptr,
                                          mRuleProcessor);
}
#endif

void
nsXBLPrototypeResources::ComputeServoStyles(const ServoStyleSet& aMasterStyleSet)
{
  mStyleRuleMap.reset(nullptr);
  mServoStyles.reset(Servo_AuthorStyles_Create());
  for (auto& sheet : mStyleSheetList) {
    Servo_AuthorStyles_AppendStyleSheet(mServoStyles.get(), sheet->AsServo());
  }
  Servo_AuthorStyles_Flush(mServoStyles.get(), aMasterStyleSet.RawSet());
}

ServoStyleRuleMap*
nsXBLPrototypeResources::GetServoStyleRuleMap()
{
  if (!HasStyleSheets() || !mServoStyles) {
    return nullptr;
  }

  if (!mStyleRuleMap) {
    mStyleRuleMap = MakeUnique<ServoStyleRuleMap>();
  }

  mStyleRuleMap->EnsureTable(*this);
  return mStyleRuleMap.get();
}

void
nsXBLPrototypeResources::AppendStyleSheet(StyleSheet* aSheet)
{
  mStyleSheetList.AppendElement(aSheet);
}

void
nsXBLPrototypeResources::RemoveStyleSheet(StyleSheet* aSheet)
{
  mStyleSheetList.RemoveElement(aSheet);
}

void
nsXBLPrototypeResources::InsertStyleSheetAt(size_t aIndex, StyleSheet* aSheet)
{
  mStyleSheetList.InsertElementAt(aIndex, aSheet);
}

void
nsXBLPrototypeResources::AppendStyleSheetsTo(
                                      nsTArray<StyleSheet*>& aResult) const
{
  aResult.AppendElements(mStyleSheetList);
}
